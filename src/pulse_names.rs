//! Small, server-independent helpers for matching PulseAudio names to streams.

/// Resolves a possibly absent server default name to the matching stream index.
/// A missing default is a valid server state and must not panic the UI callback.
pub fn resolve_default_index<'a, I>(name: Option<&str>, streams: I) -> Option<u32>
where
    I: IntoIterator<Item = (u32, &'a str)>,
{
    let name = name?;
    streams.into_iter().find_map(|(index, stream_name)| {
        if stream_name == name {
            Some(index)
        } else {
            None
        }
    })
}

#[cfg(test)]
mod tests {
    use super::resolve_default_index;

    #[test]
    fn absent_default_has_no_matching_index() {
        let streams = vec![(7, "output"), (12, "input")];
        assert_eq!(resolve_default_index(None, streams), None);
    }

    #[test]
    fn named_default_resolves_exact_stream() {
        let streams = vec![(7, "output"), (12, "input")];
        assert_eq!(resolve_default_index(Some("input"), streams), Some(12));
    }

    #[test]
    fn unknown_default_does_not_select_an_unrelated_stream() {
        let streams = vec![(7, "output")];
        assert_eq!(resolve_default_index(Some("missing"), streams), None);
    }
}
