use crate::config::Settings;
use crate::rule::RuleDescriptor;
use crate::rules;

pub fn render(settings: &Settings, enabled: &[RuleDescriptor]) -> anyhow::Result<String> {
    let scope = settings
        .paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    let mut output = format!(
        "Straitjacket repository policy\nScope: {scope}\nThese constraints are enforced in CI and before commit when the Straitjacket git hook is enabled.\n"
    );
    for descriptor in enabled {
        output.push_str("- ");
        output.push_str(&rules::instruction(descriptor.id, settings)?);
        output.push('\n');
    }
    output.push_str("Run `straitjacket` before committing. Violations fail the check.\n");
    Ok(output)
}

#[cfg(test)]
mod tests {
    use crate::config::Settings;
    use crate::scanner::Scanner;

    use super::render;

    #[test]
    fn describes_only_the_resolved_policy() {
        let settings = Settings {
            max_lines: 90,
            file_size_exclude: vec!["notes/".into()],
            theme_files: vec!["src/theme.css".into()],
            max_nesting: 3,
            no_comments: true,
            skip: vec!["emoji".into(), "motion".into()],
            ..Settings::default()
        };
        let scanner = Scanner::new(&settings).unwrap();
        let output = render(&settings, &scanner.enabled_descriptors()).unwrap();
        assert!(output.contains("Files over 90 lines"));
        assert!(output.contains("outside notes/"));
        assert!(output.contains("deeper than 3 indentation levels"));
        assert!(output.contains("Source comments are not allowed"));
        assert!(output.contains("Hardcoded colors are not allowed outside src/theme.css"));
        assert!(!output.contains("emoji"));
        assert!(!output.contains("motion"));
    }
}
