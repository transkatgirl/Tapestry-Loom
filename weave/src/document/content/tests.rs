#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::too_many_lines)]

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

#[test]
fn tokencontent_annotations() {
    let metadata_content = Some(HashMap::from([("key".to_string(), "value".to_string())]));
    let metadata_token_1 = Some(HashMap::from([("token".to_string(), "one".to_string())]));
    let metadata_token_2 = Some(HashMap::from([("token".to_string(), "two".to_string())]));
    let metadata_token_3 = Some(HashMap::from([("token".to_string(), "three".to_string())]));
    assert_eq!(
        TokenContent {
            content: vec![],
            model: None,
            metadata: metadata_content.clone(),
        }
        .annotations()
        .collect::<Vec<_>>(),
        vec![]
    );
    assert_eq!(
        TokenContent {
            content: vec![ContentToken {
                content: vec![],
                metadata: metadata_token_1.clone()
            }],
            model: None,
            metadata: metadata_content.clone(),
        }
        .annotations()
        .collect::<Vec<_>>(),
        vec![ContentAnnotation {
            range: Range { start: 0, end: 0 },
            metadata: metadata_token_1.as_ref()
        }]
    );
    assert_eq!(
        TokenContent {
            content: vec![ContentToken {
                content: vec![4, 4, 4, 4, 4, 4],
                metadata: metadata_token_1.clone()
            }],
            model: None,
            metadata: metadata_content.clone(),
        }
        .annotations()
        .collect::<Vec<_>>(),
        vec![ContentAnnotation {
            range: Range { start: 0, end: 6 },
            metadata: metadata_token_1.as_ref()
        }]
    );
    assert_eq!(
        TokenContent {
            content: vec![
                ContentToken {
                    content: vec![4, 4, 4, 4, 4, 4],
                    metadata: metadata_token_1.clone()
                },
                ContentToken {
                    content: vec![5, 5, 5, 5],
                    metadata: metadata_token_2.clone()
                }
            ],
            model: None,
            metadata: metadata_content.clone(),
        }
        .annotations()
        .collect::<Vec<_>>(),
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 6 },
                metadata: metadata_token_1.as_ref()
            },
            ContentAnnotation {
                range: Range { start: 6, end: 10 },
                metadata: metadata_token_2.as_ref()
            }
        ]
    );
    assert_eq!(
        TokenContent {
            content: vec![
                ContentToken {
                    content: vec![5, 5, 5, 5],
                    metadata: metadata_token_1.clone()
                },
                ContentToken {
                    content: vec![4, 4, 4, 4, 4, 4],
                    metadata: metadata_token_2.clone()
                },
                ContentToken {
                    content: vec![6, 6],
                    metadata: metadata_token_3.clone()
                },
            ],
            model: None,
            metadata: metadata_content.clone(),
        }
        .annotations()
        .collect::<Vec<_>>(),
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: metadata_token_1.as_ref()
            },
            ContentAnnotation {
                range: Range { start: 4, end: 10 },
                metadata: metadata_token_2.as_ref()
            },
            ContentAnnotation {
                range: Range { start: 10, end: 12 },
                metadata: metadata_token_3.as_ref()
            }
        ]
    );
}

#[test]
fn tokencontent_blank_split() {
    let metadata_content = Some(HashMap::from([("key".to_string(), "value".to_string())]));
    let metadata_token_1 = Some(HashMap::from([("token".to_string(), "one".to_string())]));
    let metadata_token_2 = Some(HashMap::from([("token".to_string(), "two".to_string())]));
    let content_no_tokens = TokenContent {
        content: vec![],
        model: None,
        metadata: metadata_content.clone(),
    };
    let content_blank_tokens = TokenContent {
        content: vec![
            ContentToken {
                content: vec![],
                metadata: metadata_token_1.clone(),
            },
            ContentToken {
                content: vec![],
                metadata: metadata_token_2.clone(),
            },
        ],
        model: None,
        metadata: metadata_content.clone(),
    };
    assert_eq!(
        content_no_tokens.clone().split(0),
        Some((content_no_tokens.clone(), content_no_tokens.clone()))
    );
    assert_eq!(content_no_tokens.clone().split(1), None);
    assert_eq!(
        content_blank_tokens.clone().split(0),
        Some((content_blank_tokens.clone(), content_no_tokens.clone()))
    );
    assert_eq!(content_blank_tokens.clone().split(1), None);
    assert_eq!(content_blank_tokens.clone().split(2), None);
    assert_eq!(content_blank_tokens.clone().split(3), None);
    assert_eq!(content_blank_tokens.clone().split(4), None);
}

#[test]
fn tokencontent_single_token_split() {
    let metadata_content = Some(HashMap::from([("key".to_string(), "value".to_string())]));
    let metadata_token = Some(HashMap::from([("token".to_string(), "one".to_string())]));
    let content = TokenContent {
        content: vec![ContentToken {
            content: vec![1, 2, 3, 1, 2],
            metadata: metadata_token.clone(),
        }],
        model: None,
        metadata: metadata_content.clone(),
    };
    assert_eq!(
        content.clone().split(0),
        Some((
            TokenContent {
                content: vec![],
                model: None,
                metadata: metadata_content.clone(),
            },
            content.clone()
        ))
    );
    assert_eq!(
        content.clone().split(1),
        Some((
            TokenContent {
                content: vec![ContentToken {
                    content: vec![1],
                    metadata: metadata_token.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![ContentToken {
                    content: vec![2, 3, 1, 2],
                    metadata: metadata_token.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(
        content.clone().split(2),
        Some((
            TokenContent {
                content: vec![ContentToken {
                    content: vec![1, 2],
                    metadata: metadata_token.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![ContentToken {
                    content: vec![3, 1, 2],
                    metadata: metadata_token.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(
        content.clone().split(3),
        Some((
            TokenContent {
                content: vec![ContentToken {
                    content: vec![1, 2, 3],
                    metadata: metadata_token.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![ContentToken {
                    content: vec![1, 2],
                    metadata: metadata_token.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(
        content.clone().split(4),
        Some((
            TokenContent {
                content: vec![ContentToken {
                    content: vec![1, 2, 3, 1],
                    metadata: metadata_token.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![ContentToken {
                    content: vec![2],
                    metadata: metadata_token.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(
        content.clone().split(5),
        Some((
            content.clone(),
            TokenContent {
                content: vec![],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(content.clone().split(6), None);
    assert_eq!(content.clone().split(7), None);
}

#[test]
fn tokencontent_multiple_tokens_split() {
    let metadata_content = Some(HashMap::from([("key".to_string(), "value".to_string())]));
    let metadata_token_1 = Some(HashMap::from([("token".to_string(), "one".to_string())]));
    let metadata_token_2 = Some(HashMap::from([("token".to_string(), "two".to_string())]));
    let metadata_token_3 = Some(HashMap::from([("token".to_string(), "three".to_string())]));
    let content = TokenContent {
        content: vec![
            ContentToken {
                content: vec![5, 5, 5],
                metadata: metadata_token_1.clone(),
            },
            ContentToken {
                content: vec![4, 4, 4, 4],
                metadata: metadata_token_2.clone(),
            },
            ContentToken {
                content: vec![6, 6],
                metadata: metadata_token_3.clone(),
            },
        ],
        model: None,
        metadata: metadata_content.clone(),
    };
    assert_eq!(
        content.clone().split(0),
        Some((
            TokenContent {
                content: vec![],
                model: None,
                metadata: metadata_content.clone(),
            },
            content.clone()
        ))
    );
    assert_eq!(
        content.clone().split(1),
        Some((
            TokenContent {
                content: vec![ContentToken {
                    content: vec![5],
                    metadata: metadata_token_1.clone(),
                },],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![
                    ContentToken {
                        content: vec![5, 5],
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: vec![4, 4, 4, 4],
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: vec![6, 6],
                        metadata: metadata_token_3.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(
        content.clone().split(2),
        Some((
            TokenContent {
                content: vec![ContentToken {
                    content: vec![5, 5],
                    metadata: metadata_token_1.clone(),
                },],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![
                    ContentToken {
                        content: vec![5],
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: vec![4, 4, 4, 4],
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: vec![6, 6],
                        metadata: metadata_token_3.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(
        content.clone().split(3),
        Some((
            TokenContent {
                content: vec![ContentToken {
                    content: vec![5, 5, 5],
                    metadata: metadata_token_1.clone(),
                },],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![
                    ContentToken {
                        content: vec![4, 4, 4, 4],
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: vec![6, 6],
                        metadata: metadata_token_3.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(
        content.clone().split(4),
        Some((
            TokenContent {
                content: vec![
                    ContentToken {
                        content: vec![5, 5, 5],
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: vec![4],
                        metadata: metadata_token_2.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![
                    ContentToken {
                        content: vec![4, 4, 4],
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: vec![6, 6],
                        metadata: metadata_token_3.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(
        content.clone().split(5),
        Some((
            TokenContent {
                content: vec![
                    ContentToken {
                        content: vec![5, 5, 5],
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: vec![4, 4],
                        metadata: metadata_token_2.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![
                    ContentToken {
                        content: vec![4, 4],
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: vec![6, 6],
                        metadata: metadata_token_3.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(
        content.clone().split(6),
        Some((
            TokenContent {
                content: vec![
                    ContentToken {
                        content: vec![5, 5, 5],
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: vec![4, 4, 4],
                        metadata: metadata_token_2.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![
                    ContentToken {
                        content: vec![4],
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: vec![6, 6],
                        metadata: metadata_token_3.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(
        content.clone().split(7),
        Some((
            TokenContent {
                content: vec![
                    ContentToken {
                        content: vec![5, 5, 5],
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: vec![4, 4, 4, 4],
                        metadata: metadata_token_2.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![ContentToken {
                    content: vec![6, 6],
                    metadata: metadata_token_3.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(
        content.clone().split(8),
        Some((
            TokenContent {
                content: vec![
                    ContentToken {
                        content: vec![5, 5, 5],
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: vec![4, 4, 4, 4],
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: vec![6],
                        metadata: metadata_token_3.clone(),
                    }
                ],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![ContentToken {
                    content: vec![6],
                    metadata: metadata_token_3.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(
        content.clone().split(9),
        Some((
            content.clone(),
            TokenContent {
                content: vec![],
                model: None,
                metadata: metadata_content.clone(),
            }
        ))
    );
    assert_eq!(content.clone().split(10), None);
    assert_eq!(content.clone().split(11), None);
    assert_eq!(content.clone().split(12), None);
    assert_eq!(content.clone().split(13), None);
    assert_eq!(content.clone().split(14), None);
}

#[test]
fn nodecontent_reduce() {
    let metadata = Some(HashMap::from([("key".to_string(), "value".to_string())]));
    assert_eq!(NodeContent::Blank.reduce(), NodeContent::Blank);
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: vec![],
            model: None,
            metadata: None
        })
        .reduce(),
        NodeContent::Blank
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: vec![],
            model: None,
            metadata: metadata.clone(),
        })
        .reduce(),
        NodeContent::Snippet(SnippetContent {
            content: vec![],
            model: None,
            metadata: metadata.clone(),
        })
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![],
            model: None,
            metadata: None
        })
        .reduce(),
        NodeContent::Blank
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![
                ContentToken {
                    content: vec![],
                    metadata: None,
                },
                ContentToken {
                    content: vec![],
                    metadata: None,
                }
            ],
            model: None,
            metadata: None
        })
        .reduce(),
        NodeContent::Blank
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![],
            model: None,
            metadata: None,
        })
        .reduce(),
        NodeContent::Blank
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![],
            model: None,
            metadata: metadata.clone(),
        })
        .reduce(),
        NodeContent::Snippet(SnippetContent {
            content: vec![],
            model: None,
            metadata: metadata.clone(),
        })
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![ContentToken {
                content: vec![],
                metadata: None,
            }],
            model: None,
            metadata: metadata.clone(),
        })
        .reduce(),
        NodeContent::Snippet(SnippetContent {
            content: vec![],
            model: None,
            metadata: metadata.clone(),
        })
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![ContentToken {
                content: vec![],
                metadata: metadata.clone(),
            }],
            model: None,
            metadata: metadata.clone(),
        })
        .reduce(),
        NodeContent::Tokens(TokenContent {
            content: vec![ContentToken {
                content: vec![],
                metadata: metadata.clone(),
            }],
            model: None,
            metadata: metadata.clone(),
        })
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: vec![],
            model: None,
            metadata: None,
        })
        .reduce(),
        NodeContent::Blank
    );
    assert_eq!(
        NodeContent::Diff(DiffContent {
            content: Diff { content: vec![] },
            model: None,
            metadata: None
        })
        .reduce(),
        NodeContent::Blank
    );
    assert_eq!(
        NodeContent::Diff(DiffContent {
            content: Diff { content: vec![] },
            model: None,
            metadata: metadata.clone(),
        })
        .reduce(),
        NodeContent::Diff(DiffContent {
            content: Diff { content: vec![] },
            model: None,
            metadata: metadata.clone(),
        })
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: vec![1, 2, 3, 4],
            model: None,
            metadata: None,
        })
        .reduce(),
        NodeContent::Snippet(SnippetContent {
            content: vec![1, 2, 3, 4],
            model: None,
            metadata: None,
        })
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![ContentToken {
                content: vec![1, 2, 3, 4],
                metadata: None,
            }],
            model: None,
            metadata: None,
        })
        .reduce(),
        NodeContent::Snippet(SnippetContent {
            content: vec![1, 2, 3, 4],
            model: None,
            metadata: None,
        })
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![
                ContentToken {
                    content: vec![1, 2, 3, 4],
                    metadata: None,
                },
                ContentToken {
                    content: vec![5, 6],
                    metadata: None,
                }
            ],
            model: None,
            metadata: None,
        })
        .reduce(),
        NodeContent::Tokens(TokenContent {
            content: vec![
                ContentToken {
                    content: vec![1, 2, 3, 4],
                    metadata: None,
                },
                ContentToken {
                    content: vec![5, 6],
                    metadata: None,
                }
            ],
            model: None,
            metadata: None,
        })
    );
    assert_eq!(
        NodeContent::Diff(DiffContent {
            content: Diff {
                content: vec![Modification {
                    index: 2,
                    content: ModificationContent::Deletion(4),
                }]
            },
            model: None,
            metadata: None,
        })
        .reduce(),
        NodeContent::Diff(DiffContent {
            content: Diff {
                content: vec![Modification {
                    index: 2,
                    content: ModificationContent::Deletion(4),
                }]
            },
            model: None,
            metadata: None,
        })
    );
}

/*#[test]
fn nodecontent_into_diff() {}*/

/*#[test]
fn nodecontent_split() {}*/

/*#[test]
fn nodecontent_merge() {}*/

#[test]
fn diff_new() {
    assert_eq!(
        Diff::new(&[], &[], Instant::now() + Duration::from_secs(60)),
        Diff { content: vec![] }
    );
    assert_eq!(
        Diff::new(
            &[1, 5, 2, 2, 3, 1, 4, 1],
            &[1, 5, 2, 2, 3, 1, 4, 1],
            Instant::now() + Duration::from_secs(60)
        ),
        Diff { content: vec![] }
    );
    assert_eq!(
        Diff::new(
            &[1, 1, 1, 1, 1, 1, 1, 1],
            &[1, 1, 1, 1],
            Instant::now() + Duration::from_secs(60)
        ),
        Diff {
            content: vec![Modification {
                index: 4,
                content: ModificationContent::Deletion(4)
            }],
        }
    );
    assert_eq!(
        Diff::new(
            &[1, 1, 1, 1, 1, 1, 1, 1],
            &[1, 1, 2, 2, 2, 2, 1, 1],
            Instant::now() + Duration::from_secs(60)
        ),
        Diff {
            content: vec![
                Modification {
                    index: 2,
                    content: ModificationContent::Deletion(4)
                },
                Modification {
                    index: 2,
                    content: ModificationContent::Insertion(vec![2, 2, 2, 2])
                }
            ],
        }
    );
    assert_eq!(
        Diff::new(
            &[1, 1, 1, 1, 1, 1, 1, 1],
            &[1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 1, 1],
            Instant::now() + Duration::from_secs(60)
        ),
        Diff {
            content: vec![Modification {
                index: 6,
                content: ModificationContent::Insertion(vec![2, 2, 2, 2])
            }],
        }
    );
}

#[test]
fn diff_apply() {
    {
        let mut before = vec![];
        let after = vec![];
        let diff = Diff::new(&before, &after, Instant::now() + Duration::from_secs(60));
        diff.apply(&mut before);
        assert_eq!(before, after);
    }
    {
        let mut before = vec![1, 5, 2, 2, 3, 1, 4, 1];
        let after = vec![1, 5, 2, 2, 3, 1, 4, 1];
        let diff = Diff::new(&before, &after, Instant::now() + Duration::from_secs(60));
        diff.apply(&mut before);
        assert_eq!(before, after);
    }
    {
        let mut before = vec![1, 1, 1, 1, 1, 1, 1, 1];
        let after = vec![1, 1, 1, 1];
        let diff = Diff::new(&before, &after, Instant::now() + Duration::from_secs(60));
        diff.apply(&mut before);
        assert_eq!(before, after);
    }
    {
        let mut before = vec![1, 1, 1, 1, 1, 1, 1, 1];
        let after = vec![1, 1, 2, 2, 2, 2, 1, 1];
        let diff = Diff::new(&before, &after, Instant::now() + Duration::from_secs(60));
        diff.apply(&mut before);
        assert_eq!(before, after);
    }
    {
        let mut before = vec![1, 1, 1, 1, 1, 1, 1, 1];
        let after = vec![1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 1, 1];
        let diff = Diff::new(&before, &after, Instant::now() + Duration::from_secs(60));
        diff.apply(&mut before);
        assert_eq!(before, after);
    }
}

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

/*#[test]
fn modification_range_apply_annotations() {}*/

/*#[test]
fn weave_timeline_annotated_string() {}*/

/*#[test]
fn weave_timeline_ranged_string() {}*/

/*#[test]
fn diff_apply_timeline_annotations() {}*/
