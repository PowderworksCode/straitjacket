//! Treebank wasm packs: loaded, called, and materialised into a tree beamte
//! can walk.
//!
//! Straitjacket owns the engine because it is about to have several structural
//! rules, not one. If each analysis library carried its own wasmer, a single
//! scan would stand up two engines and JIT the same pack twice.
//!
//! A pack is a standalone module: the tree-sitter runtime, one grammar and the
//! `tb_*` ABI, statically linked, importing only WASI. It is not linked C, so
//! it cannot break the musl cross-build the release depends on.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;
use std::sync::Mutex;

use anyhow::{Context, Result, bail};
use beamte::RoleTable;
use beamte::node::{Node, Span};
use beamte::role::RoleSet;
use wasmer::{Function, Instance, Memory, Module, Store, TypedFunction, Value};

/// The oldest pack ABI this host can drive. A pack states its own with
/// `tb_pack_abi()`, and one older than this is refused rather than called:
/// the exports it is missing would otherwise surface as unrelated errors
/// much later.
///
/// A minimum rather than an equality, because a pack is a versioned contract
/// and the version only moves forward. ABI 3 added the query exports, which
/// this host does not bind; requiring exactly 2 refused every pack treebank
/// publishes today while claiming the pack was the thing out of date.
const MINIMUM_PACK_ABI: i32 = 2;

/// `tb_node_flags` bit for a named node. The rest (error, missing, extra) are
/// not needed here.
const FLAG_NAMED: u32 = 1;

/// WASI errno 8, `badf`. The pack links libc and so imports six file-descriptor
/// calls, but it never opens anything: it is handed source bytes through
/// `tb_alloc` and hands back a tree. Stubbing them is what lets this host skip
/// a WASI implementation entirely; a pack that genuinely tried to read a file
/// would get a clean refusal rather than silence.
const WASI_BADF: i64 = 8;

struct Exports {
    initialize: TypedFunction<(), ()>,
    language_name: TypedFunction<(), i32>,
    strlen: TypedFunction<i32, u32>,
    roles: TypedFunction<(), i32>,
    roles_len: TypedFunction<(), u32>,
    node_types: TypedFunction<(), i32>,
    node_types_len: TypedFunction<(), u32>,
    alloc: TypedFunction<u32, i32>,
    free: TypedFunction<i32, ()>,
    parse: TypedFunction<(i32, u32), i32>,
    tree_free: TypedFunction<i32, ()>,
    tree_root: TypedFunction<(i32, i32), ()>,
    node_new: TypedFunction<(), i32>,
    node_free: TypedFunction<i32, ()>,
    node_child: TypedFunction<(i32, u32, i32), i32>,
    node_child_count: TypedFunction<i32, u32>,
    node_type: TypedFunction<i32, i32>,
    node_field_name_for_child: TypedFunction<(i32, u32), i32>,
    node_flags: TypedFunction<i32, u32>,
    node_start_byte: TypedFunction<i32, u32>,
    node_end_byte: TypedFunction<i32, u32>,
    node_start_row: TypedFunction<i32, u32>,
    node_start_column: TypedFunction<i32, u32>,
}

struct Loaded {
    store: Store,
    memory: Memory,
    exports: Exports,
}

/// One grammar, ready to parse.
pub struct Pack {
    loaded: Mutex<Loaded>,
    roles: RoleTable,
    language: String,
}

impl Pack {
    /// Load a pack from a `.wasm` file.
    pub fn load(path: &Path) -> Result<Pack> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("reading the grammar pack {}", path.display()))?;
        Pack::from_bytes(&bytes, &path.display().to_string())
    }

    pub fn from_bytes(bytes: &[u8], origin: &str) -> Result<Pack> {
        let mut store = Store::default();
        let module = Module::new(&store, bytes)
            .with_context(|| format!("{origin} is not a loadable wasm module"))?;

        let mut imports = wasmer::Imports::new();
        for import in module.imports() {
            if import.module() != "wasi_snapshot_preview1" {
                bail!(
                    "{origin} imports {}::{}, but a pack may import only WASI",
                    import.module(),
                    import.name()
                );
            }
            let signature = match import.ty() {
                wasmer::ExternType::Function(signature) => signature.clone(),
                other => bail!("{origin} imports a non-function {other:?}"),
            };
            imports.define(
                import.module(),
                import.name(),
                Function::new(&mut store, &signature, refuse_file_descriptors),
            );
        }

        let instance = Instance::new(&mut store, &module, &imports)
            .with_context(|| format!("instantiating {origin}"))?;
        let memory = instance
            .exports
            .get_memory("memory")
            .with_context(|| format!("{origin} exports no memory"))?
            .clone();
        check_pack_abi(&instance, &mut store, origin)?;

        let exports = Exports::from(&instance, &store, origin)?;

        start_reactor(&exports, &mut store)?;

        let mut loaded = Loaded {
            store,
            memory,
            exports,
        };
        let language = loaded.language_name()?;
        let node_types = loaded.node_types_json()?;
        let roles_json = loaded.roles_json()?;
        let roles = RoleTable::from_manifests(&node_types, &roles_json)
            .map_err(|error| anyhow::anyhow!("{origin}: {error}"))?;

        Ok(Pack {
            loaded: Mutex::new(loaded),
            roles,
            language,
        })
    }

    /// The grammar's own name for itself, such as `python`.
    pub fn language(&self) -> &str {
        &self.language
    }

    /// Terms the pack declares that beamte has no role for. Empty is healthy.
    pub fn unknown_terms(&self) -> &[String] {
        self.roles.unknown_terms()
    }

    /// Parse source into a tree that can be walked without touching wasm again.
    pub fn parse(&self, source: &str) -> Result<Tree> {
        let mut loaded = self
            .loaded
            .lock()
            .map_err(|_| anyhow::anyhow!("the grammar pack lock was poisoned"))?;
        loaded.parse(source, &self.roles)
    }
}

/// Run a pack's self-initialisation.
///
/// A pack is a WASI *reactor*: it has no `main`, it initialises itself through
/// this export, and no other export may be called before it.
/// Refuse a pack this host cannot drive, before its exports are bound.
///
/// Called first, and on its own. Binding the whole ABI before checking it
/// reports an old pack as missing some export whose name means nothing to
/// the reader, when the useful thing to say is which version it is.
///
/// `tb_pack_abi` is the one export every version of the ABI has had, and it
/// is callable before `_initialize`: it returns a constant compiled into the
/// pack rather than anything the runtime sets up.
fn check_pack_abi(instance: &Instance, store: &mut Store, origin: &str) -> Result<()> {
    let pack_abi: TypedFunction<(), i32> = instance
        .exports
        .get_typed_function(store, "tb_pack_abi")
        .with_context(|| format!("{origin} does not export tb_pack_abi; is it a treebank pack?"))?;

    let abi = pack_abi.call(store)?;
    if abi < MINIMUM_PACK_ABI {
        bail!(
            "{origin} is pack ABI {abi}, and this host needs at least \
             {MINIMUM_PACK_ABI}. Rebuild or re-fetch the pack."
        );
    }
    Ok(())
}

fn start_reactor(exports: &Exports, store: &mut Store) -> Result<()> {
    exports.initialize.call(store)?;
    Ok(())
}

/// Whether `tb_node_flags` marks this node named.
///
/// Anonymous nodes are the punctuation a grammar needs to parse and that no
/// rule here reads, so they are dropped on the way into the arena.
fn is_named(flags: u32) -> bool {
    flags & FLAG_NAMED != 0
}

/// Every WASI import a pack declares, answered with `badf`.
fn refuse_file_descriptors(_: &[Value]) -> std::result::Result<Vec<Value>, wasmer::RuntimeError> {
    Ok(vec![Value::I32(WASI_BADF as i32)])
}

impl Loaded {
    fn language_name(&mut self) -> Result<String> {
        let ptr = self.exports.language_name.call(&mut self.store)?;
        self.read_c_string_at(ptr)
    }

    fn node_types_json(&mut self) -> Result<String> {
        let ptr = self.exports.node_types.call(&mut self.store)?;
        let len = self.exports.node_types_len.call(&mut self.store)?;
        self.read_blob(ptr, len)
    }

    fn roles_json(&mut self) -> Result<String> {
        let ptr = self.exports.roles.call(&mut self.store)?;
        let len = self.exports.roles_len.call(&mut self.store)?;
        self.read_blob(ptr, len)
    }

    fn read_c_string_at(&mut self, ptr: i32) -> Result<String> {
        let len = self.exports.strlen.call(&mut self.store, ptr)?;
        self.read_blob(ptr, len)
    }

    fn read_blob(&mut self, ptr: i32, len: u32) -> Result<String> {
        let view = self.memory.view(&self.store);
        let mut buffer = vec![0u8; len as usize];
        view.read(ptr as u64, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }

    fn parse(&mut self, source: &str, roles: &RoleTable) -> Result<Tree> {
        let bytes = source.as_bytes();
        let source_ptr = self
            .exports
            .alloc
            .call(&mut self.store, bytes.len() as u32)?;
        if source_ptr == 0 {
            bail!(
                "the pack could not allocate {} bytes for the source",
                bytes.len()
            );
        }
        self.memory
            .view(&self.store)
            .write(source_ptr as u64, bytes)?;
        let tree = self
            .exports
            .parse
            .call(&mut self.store, source_ptr, bytes.len() as u32)?;
        self.exports.free.call(&mut self.store, source_ptr)?;
        if tree == 0 {
            bail!("the pack returned no tree");
        }

        let built = self.materialise(tree, source, roles);
        self.exports.tree_free.call(&mut self.store, tree)?;
        built
    }

    /// Copy the wasm tree into an arena.
    ///
    /// Rules walk a tree many times over, and every cross-module call costs
    /// more than the same read from Rust memory. Doing it once here also gives
    /// beamte the `Copy` nodes its trait wants, which a wasm handle that has to
    /// be freed cannot be.
    ///
    /// Breadth-first, because that is what makes each node's children
    /// contiguous in the arena.
    fn materialise(&mut self, tree: i32, source: &str, roles: &RoleTable) -> Result<Tree> {
        let mut arena = Tree {
            nodes: Vec::new(),
            kinds: Vec::new(),
            fields: Vec::new(),
            source: source.to_string(),
        };
        let mut kind_ids: HashMap<String, u32> = HashMap::new();
        let mut field_ids: HashMap<String, u32> = HashMap::new();

        let root_slot = self.exports.node_new.call(&mut self.store)?;
        if root_slot == 0 {
            bail!("the pack could not allocate a node");
        }
        self.exports
            .tree_root
            .call(&mut self.store, tree, root_slot)?;

        let root = self.read_node(root_slot, None, roles, &mut arena, &mut kind_ids)?;
        arena.nodes.push(root);

        let mut slots = vec![root_slot];
        let mut queue = std::collections::VecDeque::from([(0usize, root_slot)]);

        while let Some((index, slot)) = queue.pop_front() {
            let total = self.exports.node_child_count.call(&mut self.store, slot)?;
            let first_child = arena.nodes.len() as u32;
            let mut named = 0u32;
            let mut children = Vec::new();

            for position_among_all_children in 0..total {
                let child_slot = self.exports.node_new.call(&mut self.store)?;
                if child_slot == 0 {
                    bail!("the pack could not allocate a node");
                }
                let found = self.exports.node_child.call(
                    &mut self.store,
                    slot,
                    position_among_all_children,
                    child_slot,
                )?;
                if found == 0 {
                    self.exports.node_free.call(&mut self.store, child_slot)?;
                    continue;
                }
                let flags = self.exports.node_flags.call(&mut self.store, child_slot)?;
                if !is_named(flags) {
                    self.exports.node_free.call(&mut self.store, child_slot)?;
                    continue;
                }
                let field = match self.field_name_at(slot, position_among_all_children)? {
                    Some(name) => Some(intern(&mut arena.fields, &mut field_ids, name)),
                    None => None,
                };

                let node = self.read_node(child_slot, field, roles, &mut arena, &mut kind_ids)?;
                arena.nodes.push(node);
                children.push(child_slot);
                slots.push(child_slot);
                named += 1;
            }

            arena.nodes[index].first_child = first_child;
            arena.nodes[index].child_count = named;
            for (offset, child_slot) in children.into_iter().enumerate() {
                queue.push_back((first_child as usize + offset, child_slot));
            }
        }

        for slot in slots {
            self.exports.node_free.call(&mut self.store, slot)?;
        }
        Ok(arena)
    }

    /// The grammar field a child fills in its parent, if any.
    ///
    /// `position` indexes over ALL children, named and anonymous alike,
    /// because `tb_node_field_name_for_child` wraps tree-sitter's
    /// `ts_node_field_name_for_child`, which does. Asking with a named-child
    /// index returns the wrong field, or none, and does so silently.
    fn field_name_at(&mut self, parent: i32, position: u32) -> Result<Option<String>> {
        let ptr = self
            .exports
            .node_field_name_for_child
            .call(&mut self.store, parent, position)?;
        if ptr == 0 {
            return Ok(None);
        }
        Ok(Some(self.read_c_string_at(ptr)?))
    }

    fn read_node(
        &mut self,
        slot: i32,
        field: Option<u32>,
        roles: &RoleTable,
        arena: &mut Tree,
        kind_ids: &mut HashMap<String, u32>,
    ) -> Result<Raw> {
        let kind_ptr = self.exports.node_type.call(&mut self.store, slot)?;
        let kind_name = self.read_c_string_at(kind_ptr)?;
        let node_roles = roles.roles(&kind_name);
        let kind = intern(&mut arena.kinds, kind_ids, kind_name);
        Ok(Raw {
            kind,
            roles: node_roles,
            span: Span {
                start_byte: self.exports.node_start_byte.call(&mut self.store, slot)? as usize,
                end_byte: self.exports.node_end_byte.call(&mut self.store, slot)? as usize,
                line: self.exports.node_start_row.call(&mut self.store, slot)? as usize + 1,
                column: self.exports.node_start_column.call(&mut self.store, slot)? as usize + 1,
            },
            field,
            first_child: 0,
            child_count: 0,
        })
    }
}

thread_local! {
    /// Loaded packs, and the reasons for the ones that would not load.
    /// Shared by every rule that parses, so two rules meeting the same
    /// language in one scan JIT its grammar once between them.
    ///
    /// A `FileRule` must be `Send + Sync` and a wasmer `Store` is neither, so
    /// the packs cannot live in a rule. They live beside the rules instead,
    /// which costs nothing today -- the walk in `src/walk.rs` is a single
    /// sequential iterator -- and stays correct rather than unsound if that
    /// ever changes. A parallel walk would pay one JIT per thread per grammar.
    ///
    /// The failure is cached with the same weight as the success: a machine
    /// with no network pays one failed fetch, not one per file.
    static CACHED: RefCell<HashMap<&'static str, std::result::Result<Rc<Pack>, String>>> =
        RefCell::new(HashMap::new());
}

/// The pack for a grammar, fetched once and then reused.
///
/// Fetched per language, and only once a rule has already decided a file is
/// worth parsing, so a Python repository never downloads the Java grammar.
pub fn cached(grammar: &'static str) -> std::result::Result<Rc<Pack>, String> {
    CACHED.with_borrow_mut(|packs| {
        packs
            .entry(grammar)
            .or_insert_with(|| {
                acquire(grammar)
                    .map(Rc::new)
                    .map_err(|error| format!("{error:#}"))
            })
            .clone()
    })
}

fn acquire(grammar: &'static str) -> Result<Pack> {
    let bytes = treebank::fetch::fetch_bytes(grammar)?;
    Pack::from_bytes(&bytes, &format!("the treebank {grammar} pack"))
}

fn intern(table: &mut Vec<String>, ids: &mut HashMap<String, u32>, value: String) -> u32 {
    if let Some(id) = ids.get(&value) {
        return *id;
    }
    let id = table.len() as u32;
    table.push(value.clone());
    ids.insert(value, id);
    id
}

struct Raw {
    kind: u32,
    roles: RoleSet,
    span: Span,
    /// The grammar field this node fills in its parent, if any.
    field: Option<u32>,
    first_child: u32,
    child_count: u32,
}

/// A parsed file, held in Rust memory.
pub struct Tree {
    nodes: Vec<Raw>,
    kinds: Vec<String>,
    fields: Vec<String>,
    source: String,
}

impl Tree {
    pub fn root(&self) -> TreeNode<'_> {
        TreeNode {
            tree: self,
            index: 0,
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// One node of a [`Tree`]. Cheap to copy: it is an index.
#[derive(Clone, Copy)]
pub struct TreeNode<'t> {
    tree: &'t Tree,
    index: u32,
}

impl<'t> TreeNode<'t> {
    fn raw(&self) -> &'t Raw {
        &self.tree.nodes[self.index as usize]
    }
}

impl<'t> Node<'t> for TreeNode<'t> {
    fn kind(&self) -> &'t str {
        &self.tree.kinds[self.raw().kind as usize]
    }

    fn roles(&self) -> RoleSet {
        self.raw().roles
    }

    fn span(&self) -> Span {
        self.raw().span
    }

    fn child_count(&self) -> usize {
        self.raw().child_count as usize
    }

    fn child(&self, index: usize) -> Option<Self> {
        let raw = self.raw();
        (index < raw.child_count as usize).then(|| TreeNode {
            tree: self.tree,
            index: raw.first_child + index as u32,
        })
    }

    fn child_by_field(&self, name: &str) -> Option<Self> {
        let raw = self.raw();
        (0..raw.child_count).find_map(|offset| {
            let index = raw.first_child + offset;
            let field = self.tree.nodes[index as usize].field?;
            (self.tree.fields[field as usize] == name).then_some(TreeNode {
                tree: self.tree,
                index,
            })
        })
    }

    fn text(&self) -> &'t str {
        let span = self.raw().span;
        self.tree
            .source
            .get(span.start_byte..span.end_byte)
            .unwrap_or_default()
    }
}

impl Exports {
    fn from(instance: &Instance, store: &Store, origin: &str) -> Result<Exports> {
        macro_rules! typed {
            ($name:literal) => {
                instance
                    .exports
                    .get_typed_function(store, $name)
                    .with_context(|| format!("{origin} does not export {}", $name))?
            };
        }
        Ok(Exports {
            initialize: typed!("_initialize"),
            language_name: typed!("tb_language_name"),
            strlen: typed!("tb_strlen"),
            roles: typed!("tb_roles"),
            roles_len: typed!("tb_roles_len"),
            node_types: typed!("tb_node_types"),
            node_types_len: typed!("tb_node_types_len"),
            alloc: typed!("tb_alloc"),
            free: typed!("tb_free"),
            parse: typed!("tb_parse"),
            tree_free: typed!("tb_tree_free"),
            tree_root: typed!("tb_tree_root"),
            node_new: typed!("tb_node_new"),
            node_free: typed!("tb_node_free"),
            node_child: typed!("tb_node_child"),
            node_child_count: typed!("tb_node_child_count"),
            node_type: typed!("tb_node_type"),
            node_field_name_for_child: typed!("tb_node_field_name_for_child"),
            node_flags: typed!("tb_node_flags"),
            node_start_byte: typed!("tb_node_start_byte"),
            node_end_byte: typed!("tb_node_end_byte"),
            node_start_row: typed!("tb_node_start_row"),
            node_start_column: typed!("tb_node_start_column"),
        })
    }
}
