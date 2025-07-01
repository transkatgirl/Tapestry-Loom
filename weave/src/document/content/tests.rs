#![allow(clippy::should_panic_without_expect)]

use std::time::Duration;

use super::*;

#[test]
fn snippet_split() {
    let metadata = Some(HashMap::from([("key".to_string(), "value".to_string())]));
    let snippet = SnippetContent {
        content: vec![1, 2, 3, 1, 2],
        model: None,
        metadata: metadata.clone(),
    };

    assert_eq!(
        snippet.clone().split(0),
        Some((
            SnippetContent {
                content: vec![],
                model: None,
                metadata: metadata.clone(),
            },
            SnippetContent {
                content: vec![1, 2, 3, 1, 2],
                model: None,
                metadata: metadata.clone(),
            }
        ))
    );
    assert_eq!(
        snippet.clone().split(1),
        Some((
            SnippetContent {
                content: vec![1],
                model: None,
                metadata: metadata.clone(),
            },
            SnippetContent {
                content: vec![2, 3, 1, 2],
                model: None,
                metadata: metadata.clone(),
            }
        ))
    );
    assert_eq!(
        snippet.clone().split(2),
        Some((
            SnippetContent {
                content: vec![1, 2],
                model: None,
                metadata: metadata.clone(),
            },
            SnippetContent {
                content: vec![3, 1, 2],
                model: None,
                metadata: metadata.clone(),
            }
        ))
    );
    assert_eq!(
        snippet.clone().split(3),
        Some((
            SnippetContent {
                content: vec![1, 2, 3],
                model: None,
                metadata: metadata.clone(),
            },
            SnippetContent {
                content: vec![1, 2],
                model: None,
                metadata: metadata.clone(),
            }
        ))
    );
    assert_eq!(
        snippet.clone().split(4),
        Some((
            SnippetContent {
                content: vec![1, 2, 3, 1],
                model: None,
                metadata: metadata.clone(),
            },
            SnippetContent {
                content: vec![2],
                model: None,
                metadata: metadata.clone(),
            }
        ))
    );
    assert_eq!(
        snippet.clone().split(5),
        Some((
            SnippetContent {
                content: vec![1, 2, 3, 1, 2],
                model: None,
                metadata: metadata.clone(),
            },
            SnippetContent {
                content: vec![],
                model: None,
                metadata,
            }
        ))
    );
    assert_eq!(snippet.split(6), None);
}

#[test]
fn content_annotation_split() {
    let annotation = ContentAnnotation {
        range: Range { start: 2, end: 7 },
        metadata: None,
    };
    assert_eq!(annotation.clone().split(0), None);
    assert_eq!(
        annotation.clone().split(1),
        Some((
            ContentAnnotation {
                range: Range { start: 2, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 7 },
                metadata: None,
            }
        ))
    );
    assert_eq!(
        annotation.clone().split(2),
        Some((
            ContentAnnotation {
                range: Range { start: 2, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 7 },
                metadata: None,
            }
        ))
    );
    assert_eq!(
        annotation.clone().split(3),
        Some((
            ContentAnnotation {
                range: Range { start: 2, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 7 },
                metadata: None,
            }
        ))
    );
    assert_eq!(
        annotation.clone().split(4),
        Some((
            ContentAnnotation {
                range: Range { start: 2, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 7 },
                metadata: None,
            }
        ))
    );
    assert_eq!(annotation.clone().split(5), None);
}

/*#[test]
fn empty_inputs_diff() {
    let diff = Diff::new(&[], &[], Instant::now() + Duration::from_secs(60));
    assert!(diff.is_empty());
}
*/

#[test]
fn apply_modification_in_bounds() {
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
}

#[test]
#[should_panic]
fn apply_modification_out_bounds_insertion() {
    let mut content: Vec<u8> = vec![1, 1, 1, 1, 1, 1, 1, 1];
    Modification {
        index: 9,
        content: ModificationContent::Insertion(vec![5]),
    }
    .apply(&mut content);
}

#[test]
#[should_panic]
fn apply_modification_out_bounds_deletion() {
    let mut content: Vec<u8> = vec![1, 1, 1, 1, 1, 1, 1, 1];
    Modification {
        index: 9,
        content: ModificationContent::Deletion(1),
    }
    .apply(&mut content);
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
