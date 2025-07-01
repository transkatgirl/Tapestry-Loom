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

#[test]
fn format_modification_count() {
    assert_eq!(
        format!(
            "{}",
            ModificationCount {
                total: 0,
                insertions: 0,
                deletions: 0
            }
        ),
        "No Changes"
    );
    assert_eq!(
        format!(
            "{}",
            ModificationCount {
                total: 1,
                insertions: 1,
                deletions: 0
            }
        ),
        "1 Insertion"
    );
    assert_eq!(
        format!(
            "{}",
            ModificationCount {
                total: 1,
                insertions: 0,
                deletions: 1
            }
        ),
        "1 Deletion"
    );
    assert_eq!(
        format!(
            "{}",
            ModificationCount {
                total: 2,
                insertions: 1,
                deletions: 1
            }
        ),
        "1 Insertion, 1 Deletion"
    );
    assert_eq!(
        format!(
            "{}",
            ModificationCount {
                total: 2,
                insertions: 2,
                deletions: 0
            }
        ),
        "2 Insertions"
    );
    assert_eq!(
        format!(
            "{}",
            ModificationCount {
                total: 2,
                insertions: 0,
                deletions: 2
            }
        ),
        "2 Deletions"
    );
    assert_eq!(
        format!(
            "{}",
            ModificationCount {
                total: 4,
                insertions: 2,
                deletions: 2
            }
        ),
        "2 Insertions, 2 Deletions"
    );
}
