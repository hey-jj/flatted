mod common;
use common::*;

use flatted::{stringify, Replacer};

#[test]
fn allowlist_replacer_keeps_array_elements() {
    let value = obj(vec![("a", arr(vec![n(1), n(2), n(3)])), ("b", n(5))]);
    let replacer = Replacer::Allowlist(vec!["a".to_string()]);

    assert_eq!(
        stringify(&value, Some(&replacer), None),
        r#"[{"a":"1"},[1,2,3]]"#
    );
}
