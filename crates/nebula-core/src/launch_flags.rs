//! LAUNCH FLAGS: the extra CLI flags a client may ask a fresh session to
//! start with (`ClientRequest::CreateAgent::launch_flags`) — Claude's
//! `--permission-mode plan`, for one. They ride the CLI's argv, so the
//! daemon takes nothing it has not listed here: each harness has its own
//! allowlist of flags, and each flag its own list of values. Anything else
//! is refused with a reason, never passed through. Nothing here reaches a
//! shell as text: the daemon hands the normalized words to the login-shell
//! wrapper as separate, quoted arguments like the model flag.

use crate::entities::AgentKind;

/// Most words one launch may carry. Every listed flag takes one value, so
/// this is room for every allowlisted flag with headroom, and a bound on
/// what a confused client can make the daemon look at.
pub const MAX_LAUNCH_FLAG_WORDS: usize = 16;

/// One allowlisted flag: its spelling and the values it accepts.
struct Allowed {
    flag: &'static str,
    values: &'static [&'static str],
}

/// Claude's allowlist. `--permission-mode` takes the modes that keep a
/// human in the loop or narrow what the agent may do; `bypassPermissions`
/// and `dontAsk` are left out on purpose, so a client can never launch a
/// session that skips its permission prompts. `default` is the older
/// spelling of `manual`, kept for CLIs that still take it.
const CLAUDE: &[Allowed] = &[Allowed {
    flag: "--permission-mode",
    values: &["plan", "acceptEdits", "auto", "manual", "default"],
}];

fn allowlist(kind: AgentKind) -> &'static [Allowed] {
    match kind {
        AgentKind::Claude => CLAUDE,
        _ => &[],
    }
}

/// The words that start a Claude session in its native plan mode.
pub fn plan_mode() -> Vec<String> {
    vec!["--permission-mode".into(), "plan".into()]
}

/// Check `words` against `kind`'s allowlist and return them normalized:
/// every flag spelled `--flag value` (a `--flag=value` word is split), in
/// the order given. Refused with a one-line reason when a word is not an
/// allowlisted flag, a value is not one the flag takes, a flag repeats, or
/// a value is missing. An empty list is fine and stays empty.
pub fn validate(kind: AgentKind, words: &[String]) -> Result<Vec<String>, String> {
    if words.is_empty() {
        return Ok(Vec::new());
    }
    if words.len() > MAX_LAUNCH_FLAG_WORDS {
        return Err(format!(
            "too many launch flags ({} words, at most {MAX_LAUNCH_FLAG_WORDS})",
            words.len()
        ));
    }
    let allowed = allowlist(kind);
    if allowed.is_empty() {
        return Err(format!(
            "launch flags are not supported for {kind:?} sessions"
        ));
    }
    let mut out = Vec::with_capacity(words.len());
    let mut seen: Vec<&'static str> = Vec::new();
    let mut iter = words.iter();
    while let Some(word) = iter.next() {
        let word = word.trim();
        let (flag, inline) = match word.split_once('=') {
            Some((flag, value)) => (flag, Some(value)),
            None => (word, None),
        };
        let Some(entry) = allowed.iter().find(|a| a.flag == flag) else {
            return Err(format!("launch flag not allowed: {}", shown(flag)));
        };
        if seen.contains(&entry.flag) {
            return Err(format!("launch flag given twice: {}", entry.flag));
        }
        seen.push(entry.flag);
        let value = match inline {
            Some(value) => value,
            None => match iter.next() {
                Some(value) => value.trim(),
                None => return Err(format!("launch flag {} needs a value", entry.flag)),
            },
        };
        let Some(value) = entry.values.iter().find(|v| **v == value) else {
            return Err(format!(
                "{} does not take {} (one of: {})",
                entry.flag,
                shown(value),
                entry.values.join(", ")
            ));
        };
        out.push(entry.flag.to_string());
        out.push((*value).to_string());
    }
    Ok(out)
}

/// A refused word, quoted and cut short so an error line stays one line
/// and never echoes a whole pasted payload back.
fn shown(word: &str) -> String {
    const MAX: usize = 40;
    let clean: String = word
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if clean.chars().count() > MAX {
        let cut: String = clean.chars().take(MAX).collect();
        format!("`{cut}…`")
    } else {
        format!("`{clean}`")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn plan_mode_is_allowed_for_claude_in_both_spellings() {
        let want = plan_mode();
        assert_eq!(
            validate(AgentKind::Claude, &words(&["--permission-mode", "plan"])).unwrap(),
            want
        );
        assert_eq!(
            validate(AgentKind::Claude, &words(&["--permission-mode=plan"])).unwrap(),
            want
        );
        assert_eq!(
            validate(
                AgentKind::Claude,
                &words(&[" --permission-mode ", " plan "])
            )
            .unwrap(),
            want
        );
    }

    #[test]
    fn empty_is_fine_for_every_kind() {
        for kind in [AgentKind::Claude, AgentKind::Codex, AgentKind::Custom] {
            assert_eq!(validate(kind, &[]).unwrap(), Vec::<String>::new());
        }
    }

    #[test]
    fn modes_that_skip_permission_prompts_are_refused() {
        for mode in ["bypassPermissions", "dontAsk"] {
            let err =
                validate(AgentKind::Claude, &words(&["--permission-mode", mode])).unwrap_err();
            assert!(err.contains("does not take"), "{err}");
        }
        let err = validate(
            AgentKind::Claude,
            &words(&["--dangerously-skip-permissions"]),
        )
        .unwrap_err();
        assert!(err.contains("not allowed"), "{err}");
    }

    #[test]
    fn anything_shell_shaped_is_refused() {
        for bad in [
            &["--permission-mode", "plan; rm -rf ~"][..],
            &["--permission-mode", "$(whoami)"],
            &["plan"],
            &["--model", "opus"],
            &["--permission-mode=plan", "--add-dir", "/"],
            &["-p"],
            &[""],
        ] {
            assert!(
                validate(AgentKind::Claude, &words(bad)).is_err(),
                "{bad:?} should be refused"
            );
        }
    }

    #[test]
    fn repeats_and_missing_values_are_refused() {
        let err = validate(
            AgentKind::Claude,
            &words(&["--permission-mode", "plan", "--permission-mode=auto"]),
        )
        .unwrap_err();
        assert!(err.contains("twice"), "{err}");
        let err = validate(AgentKind::Claude, &words(&["--permission-mode"])).unwrap_err();
        assert!(err.contains("needs a value"), "{err}");
    }

    #[test]
    fn other_harnesses_take_no_launch_flags_yet() {
        let err = validate(AgentKind::Codex, &plan_mode()).unwrap_err();
        assert!(err.contains("not supported"), "{err}");
    }

    #[test]
    fn long_lists_and_long_words_stay_bounded() {
        let many = vec!["--permission-mode".to_string(); MAX_LAUNCH_FLAG_WORDS + 1];
        assert!(validate(AgentKind::Claude, &many)
            .unwrap_err()
            .contains("too many"));
        let long = format!("--{}", "x".repeat(500));
        let err = validate(AgentKind::Claude, &[long]).unwrap_err();
        assert!(err.len() < 120, "{err}");
    }
}
