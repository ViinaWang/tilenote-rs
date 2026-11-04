/// Canonicalize the shorthand `[] task`/`[x] task` used by the user to GFM
/// `- [ ] task`/`- [x] task`. Otherwise `egui_commonmark` has no list checkbox
/// node to click. Preserve code fences and already-canonical task lists.
/// Called at editor→preview transition, not during each keystroke.
pub fn normalize_task_shorthand(source: &str) -> String {
    let mut out = String::with_capacity(source.len() + 16);
    let mut fence: Option<(char, usize)> = None;
    for line in source.split_inclusive('\n') {
        let trim = line.trim_start_matches([' ', '\t']);
        let spaces = line.len() - trim.len();
        let leader = trim.chars().next();
        if let Some(ch @ ('`' | '~')) = leader {
            let count = trim.chars().take_while(|&c| c == ch).count();
            if count >= 3 {
                match fence {
                    None => fence = Some((ch, count)),
                    Some((open, min)) if ch == open && count >= min => fence = None,
                    _ => (),
                }
                out.push_str(line);
                continue;
            }
        }
        if fence.is_none() {
            let (prefix, rest) = if let Some(r) = trim.strip_prefix("[] ") {
                ("- [ ] ", Some(r))
            } else if let Some(r) = trim.strip_prefix("[ ] ") {
                ("- [ ] ", Some(r))
            } else if let Some(r) = trim.strip_prefix("[x] ").or_else(|| trim.strip_prefix("[X] ")) {
                ("- [x] ", Some(r))
            } else if let Some(r) = trim.strip_prefix("- [] ") {
                ("- [ ] ", Some(r))
            } else if let Some(r) = trim.strip_prefix("* [] ") {
                ("* [ ] ", Some(r))
            } else {
                ("", None)
            };
            if let Some(rest) = rest {
                out.push_str(&line[..spaces]);
                out.push_str(prefix);
                out.push_str(rest);
                continue;
            }
        }
        out.push_str(line);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shorthand_to_gfm() {
        assert_eq!(
            normalize_task_shorthand("[] draft\n[x] done\n* [] starred\n- [ ] existing\n"),
            "- [ ] draft\n- [x] done\n* [ ] starred\n- [ ] existing\n"
        );
    }
    #[test]
    fn fenced_code_is_unchanged() {
        assert_eq!(
            normalize_task_shorthand("```md\n[] code\n```\n[] real\n~~~\n[x] code\n~~~\n"),
            "```md\n[] code\n```\n- [ ] real\n~~~\n[x] code\n~~~\n"
        );
    }
    #[test]
    fn ordinary_text_is_unchanged() {
        assert_eq!(normalize_task_shorthand("[ ]task\nnot a task\n"), "[ ]task\nnot a task\n");
    }
}
