use std::time::Duration;

use super::*;

/*#[test]
fn empty_inputs_diff() {
    let diff = Diff::new(&[], &[], Instant::now() + Duration::from_secs(60));
    assert!(diff.is_empty());
}
*/

#[test]
fn apply_modification() {
    let mut content: Vec<u8> = vec![1, 1, 1, 1, 1, 1, 1, 1];
    Modification {
        index: 0,
        content: ModificationContent::Insertion(vec![2, 2, 2]),
    }
    .apply(&mut content);
    assert_eq!(content, vec![2, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1]);
    Modification {
        index: 0,
        content: ModificationContent::Deletion(3),
    }
    .apply(&mut content);
    assert_eq!(content, vec![1, 1, 1, 1, 1, 1, 1, 1]);
    Modification {
        index: 5,
        content: ModificationContent::Insertion(vec![3, 3, 3, 3]),
    }
    .apply(&mut content);
    assert_eq!(content, vec![1, 1, 1, 1, 1, 3, 3, 3, 3, 1, 1, 1]);
    Modification {
        index: 5,
        content: ModificationContent::Deletion(4),
    }
    .apply(&mut content);
    assert_eq!(content, vec![1, 1, 1, 1, 1, 1, 1, 1]);
    Modification {
        index: 8,
        content: ModificationContent::Insertion(vec![4, 4]),
    }
    .apply(&mut content);
    assert_eq!(content, vec![1, 1, 1, 1, 1, 1, 1, 1, 4, 4]);
    Modification {
        index: 8,
        content: ModificationContent::Deletion(2),
    }
    .apply(&mut content);
    assert_eq!(content, vec![1, 1, 1, 1, 1, 1, 1, 1]);
    Modification {
        index: 9,
        content: ModificationContent::Insertion(vec![5]),
    }
    .apply(&mut content);
    assert_eq!(content, vec![1, 1, 1, 1, 1, 1, 1, 1]);
    Modification {
        index: 9,
        content: ModificationContent::Deletion(1),
    }
    .apply(&mut content);
    assert_eq!(content, vec![1, 1, 1, 1, 1, 1, 1, 1]);
}
