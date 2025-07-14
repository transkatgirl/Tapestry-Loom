#![allow(clippy::should_panic_without_expect)]
#![allow(clippy::too_many_lines)]

use std::time::Duration;

use super::*;

#[test]
fn snippet_split() {
    let metadata = Some(HashMap::from([("key".to_string(), "value".to_string())]));
    let snippet = SnippetContent {
        content: Bytes::from_static(&[1, 2, 3, 1, 2]),
        model: None,
        metadata: metadata.clone(),
    };

    assert_eq!(
        snippet.clone().split(0),
        Some((
            SnippetContent {
                content: Bytes::new(),
                model: None,
                metadata: metadata.clone(),
            },
            SnippetContent {
                content: Bytes::from_static(&[1, 2, 3, 1, 2]),
                model: None,
                metadata: metadata.clone(),
            }
        ))
    );
    assert_eq!(
        snippet.clone().split(1),
        Some((
            SnippetContent {
                content: Bytes::from_static(&[1]),
                model: None,
                metadata: metadata.clone(),
            },
            SnippetContent {
                content: Bytes::from_static(&[2, 3, 1, 2]),
                model: None,
                metadata: metadata.clone(),
            }
        ))
    );
    assert_eq!(
        snippet.clone().split(2),
        Some((
            SnippetContent {
                content: Bytes::from_static(&[1, 2]),
                model: None,
                metadata: metadata.clone(),
            },
            SnippetContent {
                content: Bytes::from_static(&[3, 1, 2]),
                model: None,
                metadata: metadata.clone(),
            }
        ))
    );
    assert_eq!(
        snippet.clone().split(3),
        Some((
            SnippetContent {
                content: Bytes::from_static(&[1, 2, 3]),
                model: None,
                metadata: metadata.clone(),
            },
            SnippetContent {
                content: Bytes::from_static(&[1, 2]),
                model: None,
                metadata: metadata.clone(),
            }
        ))
    );
    assert_eq!(
        snippet.clone().split(4),
        Some((
            SnippetContent {
                content: Bytes::from_static(&[1, 2, 3, 1]),
                model: None,
                metadata: metadata.clone(),
            },
            SnippetContent {
                content: Bytes::from_static(&[2]),
                model: None,
                metadata: metadata.clone(),
            }
        ))
    );
    assert_eq!(
        snippet.clone().split(5),
        Some((
            SnippetContent {
                content: Bytes::from_static(&[1, 2, 3, 1, 2]),
                model: None,
                metadata: metadata.clone(),
            },
            SnippetContent {
                content: Bytes::new(),
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
        len: 5,
        metadata: None,
    };
    assert_eq!(annotation.clone().split(0), None);
    assert_eq!(
        annotation.clone().split(1),
        Some((
            ContentAnnotation {
                len: 1,
                metadata: None,
            },
            ContentAnnotation {
                len: 4,
                metadata: None,
            }
        ))
    );
    assert_eq!(
        annotation.clone().split(2),
        Some((
            ContentAnnotation {
                len: 2,
                metadata: None,
            },
            ContentAnnotation {
                len: 3,
                metadata: None,
            }
        ))
    );
    assert_eq!(
        annotation.clone().split(3),
        Some((
            ContentAnnotation {
                len: 3,
                metadata: None,
            },
            ContentAnnotation {
                len: 2,
                metadata: None,
            }
        ))
    );
    assert_eq!(
        annotation.clone().split(4),
        Some((
            ContentAnnotation {
                len: 4,
                metadata: None,
            },
            ContentAnnotation {
                len: 1,
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
                content: Bytes::new(),
                metadata: metadata_token_1.clone()
            }],
            model: None,
            metadata: metadata_content.clone(),
        }
        .annotations()
        .collect::<Vec<_>>(),
        vec![ContentAnnotation {
            len: 0,
            metadata: metadata_token_1.as_ref()
        }]
    );
    assert_eq!(
        TokenContent {
            content: vec![ContentToken {
                content: Bytes::from_static(&[4, 4, 4, 4, 4, 4]),
                metadata: metadata_token_1.clone()
            }],
            model: None,
            metadata: metadata_content.clone(),
        }
        .annotations()
        .collect::<Vec<_>>(),
        vec![ContentAnnotation {
            len: 6,
            metadata: metadata_token_1.as_ref()
        }]
    );
    assert_eq!(
        TokenContent {
            content: vec![
                ContentToken {
                    content: Bytes::from_static(&[4, 4, 4, 4, 4, 4]),
                    metadata: metadata_token_1.clone()
                },
                ContentToken {
                    content: Bytes::from_static(&[5, 5, 5, 5]),
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
                len: 6,
                metadata: metadata_token_1.as_ref()
            },
            ContentAnnotation {
                len: 4,
                metadata: metadata_token_2.as_ref()
            }
        ]
    );
    assert_eq!(
        TokenContent {
            content: vec![
                ContentToken {
                    content: Bytes::from_static(&[5, 5, 5, 5]),
                    metadata: metadata_token_1.clone()
                },
                ContentToken {
                    content: Bytes::from_static(&[4, 4, 4, 4, 4, 4]),
                    metadata: metadata_token_2.clone()
                },
                ContentToken {
                    content: Bytes::from_static(&[6, 6]),
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
                len: 4,
                metadata: metadata_token_1.as_ref()
            },
            ContentAnnotation {
                len: 6,
                metadata: metadata_token_2.as_ref()
            },
            ContentAnnotation {
                len: 2,
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
                content: Bytes::new(),
                metadata: metadata_token_1.clone(),
            },
            ContentToken {
                content: Bytes::new(),
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
            content: Bytes::from_static(&[1, 2, 3, 1, 2]),
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
                    content: Bytes::from_static(&[1]),
                    metadata: metadata_token.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![ContentToken {
                    content: Bytes::from_static(&[2, 3, 1, 2]),
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
                    content: Bytes::from_static(&[1, 2]),
                    metadata: metadata_token.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![ContentToken {
                    content: Bytes::from_static(&[3, 1, 2]),
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
                    content: Bytes::from_static(&[1, 2, 3]),
                    metadata: metadata_token.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![ContentToken {
                    content: Bytes::from_static(&[1, 2]),
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
                    content: Bytes::from_static(&[1, 2, 3, 1]),
                    metadata: metadata_token.clone(),
                }],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![ContentToken {
                    content: Bytes::from_static(&[2]),
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
                content: Bytes::from_static(&[5, 5, 5]),
                metadata: metadata_token_1.clone(),
            },
            ContentToken {
                content: Bytes::from_static(&[4, 4, 4, 4]),
                metadata: metadata_token_2.clone(),
            },
            ContentToken {
                content: Bytes::from_static(&[6, 6]),
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
                    content: Bytes::from_static(&[5]),
                    metadata: metadata_token_1.clone(),
                },],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[5, 5]),
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[4, 4, 4, 4]),
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[6, 6]),
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
                    content: Bytes::from_static(&[5, 5]),
                    metadata: metadata_token_1.clone(),
                },],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[5]),
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[4, 4, 4, 4]),
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[6, 6]),
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
                    content: Bytes::from_static(&[5, 5, 5]),
                    metadata: metadata_token_1.clone(),
                },],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[4, 4, 4, 4]),
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[6, 6]),
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
                        content: Bytes::from_static(&[5, 5, 5]),
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[4]),
                        metadata: metadata_token_2.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[4, 4, 4]),
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[6, 6]),
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
                        content: Bytes::from_static(&[5, 5, 5]),
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[4, 4]),
                        metadata: metadata_token_2.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[4, 4]),
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[6, 6]),
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
                        content: Bytes::from_static(&[5, 5, 5]),
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[4, 4, 4]),
                        metadata: metadata_token_2.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[4]),
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[6, 6]),
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
                        content: Bytes::from_static(&[5, 5, 5]),
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[4, 4, 4, 4]),
                        metadata: metadata_token_2.clone(),
                    },
                ],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![ContentToken {
                    content: Bytes::from_static(&[6, 6]),
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
                        content: Bytes::from_static(&[5, 5, 5]),
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[4, 4, 4, 4]),
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[6]),
                        metadata: metadata_token_3.clone(),
                    }
                ],
                model: None,
                metadata: metadata_content.clone(),
            },
            TokenContent {
                content: vec![ContentToken {
                    content: Bytes::from_static(&[6]),
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
            content: Bytes::new(),
            model: None,
            metadata: None
        })
        .reduce(),
        NodeContent::Blank
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: Bytes::new(),
            model: None,
            metadata: metadata.clone(),
        })
        .reduce(),
        NodeContent::Snippet(SnippetContent {
            content: Bytes::new(),
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
                    content: Bytes::new(),
                    metadata: None,
                },
                ContentToken {
                    content: Bytes::new(),
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
            content: Bytes::new(),
            model: None,
            metadata: metadata.clone(),
        })
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![ContentToken {
                content: Bytes::new(),
                metadata: None,
            }],
            model: None,
            metadata: metadata.clone(),
        })
        .reduce(),
        NodeContent::Snippet(SnippetContent {
            content: Bytes::new(),
            model: None,
            metadata: metadata.clone(),
        })
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![ContentToken {
                content: Bytes::new(),
                metadata: metadata.clone(),
            }],
            model: None,
            metadata: metadata.clone(),
        })
        .reduce(),
        NodeContent::Tokens(TokenContent {
            content: vec![ContentToken {
                content: Bytes::new(),
                metadata: metadata.clone(),
            }],
            model: None,
            metadata: metadata.clone(),
        })
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: Bytes::new(),
            model: None,
            metadata: None,
        })
        .reduce(),
        NodeContent::Blank
    );
    assert_eq!(
        NodeContent::Diff(DiffContent {
            content: Diff::default(),
            model: None,
            metadata: None
        })
        .reduce(),
        NodeContent::Blank
    );
    assert_eq!(
        NodeContent::Diff(DiffContent {
            content: Diff::default(),
            model: None,
            metadata: metadata.clone(),
        })
        .reduce(),
        NodeContent::Diff(DiffContent {
            content: Diff::default(),
            model: None,
            metadata: metadata.clone(),
        })
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: Bytes::from_static(&[1, 2, 3, 4]),
            model: None,
            metadata: None,
        })
        .reduce(),
        NodeContent::Snippet(SnippetContent {
            content: Bytes::from_static(&[1, 2, 3, 4]),
            model: None,
            metadata: None,
        })
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![ContentToken {
                content: Bytes::from_static(&[1, 2, 3, 4]),
                metadata: None,
            }],
            model: None,
            metadata: None,
        })
        .reduce(),
        NodeContent::Snippet(SnippetContent {
            content: Bytes::from_static(&[1, 2, 3, 4]),
            model: None,
            metadata: None,
        })
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![
                ContentToken {
                    content: Bytes::from_static(&[1, 2, 3, 4]),
                    metadata: None,
                },
                ContentToken {
                    content: Bytes::from_static(&[5, 6]),
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
                    content: Bytes::from_static(&[1, 2, 3, 4]),
                    metadata: None,
                },
                ContentToken {
                    content: Bytes::from_static(&[5, 6]),
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

#[test]
fn nodecontent_into_diff() {
    assert_eq!(
        NodeContent::Blank.into_diff(Range { start: 2, end: 5 }),
        None
    );
    assert_eq!(
        NodeContent::Diff(DiffContent {
            content: Diff {
                content: vec![Modification {
                    index: 2,
                    content: ModificationContent::Deletion(3)
                }],
            },
            model: None,
            metadata: None
        })
        .into_diff(Range { start: 2, end: 5 }),
        None
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: Bytes::new(),
            model: None,
            metadata: None
        })
        .into_diff(Range { start: 2, end: 2 }),
        Some(NodeContent::Diff(DiffContent {
            content: Diff {
                content: vec![Modification {
                    index: 2,
                    content: ModificationContent::Insertion(Bytes::new())
                }],
            },
            model: None,
            metadata: None
        }))
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: Bytes::from_static(&[4, 3, 2, 1]),
            model: None,
            metadata: None
        })
        .into_diff(Range { start: 2, end: 2 }),
        Some(NodeContent::Diff(DiffContent {
            content: Diff {
                content: vec![Modification {
                    index: 2,
                    content: ModificationContent::Insertion(Bytes::from_static(&[4, 3, 2, 1]))
                }],
            },
            model: None,
            metadata: None
        }))
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: Bytes::from_static(&[4, 3, 2, 1]),
            model: None,
            metadata: None
        })
        .into_diff(Range { start: 2, end: 5 }),
        Some(NodeContent::Diff(DiffContent {
            content: Diff {
                content: vec![
                    Modification {
                        index: 2,
                        content: ModificationContent::Deletion(3)
                    },
                    Modification {
                        index: 2,
                        content: ModificationContent::Insertion(Bytes::from_static(&[4, 3, 2, 1]))
                    }
                ],
            },
            model: None,
            metadata: None
        }))
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![],
            model: None,
            metadata: None
        })
        .into_diff(Range { start: 2, end: 2 }),
        Some(NodeContent::Diff(DiffContent {
            content: Diff {
                content: vec![Modification {
                    index: 2,
                    content: ModificationContent::TokenInsertion(vec![])
                }],
            },
            model: None,
            metadata: None
        }))
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![
                ContentToken {
                    content: Bytes::from_static(&[4, 1, 2]),
                    metadata: None
                },
                ContentToken {
                    content: Bytes::from_static(&[6, 3]),
                    metadata: None
                },
            ],
            model: None,
            metadata: None
        })
        .into_diff(Range { start: 2, end: 2 }),
        Some(NodeContent::Diff(DiffContent {
            content: Diff {
                content: vec![Modification {
                    index: 2,
                    content: ModificationContent::TokenInsertion(vec![
                        ContentToken {
                            content: Bytes::from_static(&[4, 1, 2]),
                            metadata: None
                        },
                        ContentToken {
                            content: Bytes::from_static(&[6, 3]),
                            metadata: None
                        },
                    ])
                }],
            },
            model: None,
            metadata: None
        }))
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![
                ContentToken {
                    content: Bytes::from_static(&[4, 1, 2]),
                    metadata: None
                },
                ContentToken {
                    content: Bytes::from_static(&[6, 3]),
                    metadata: None
                },
            ],
            model: None,
            metadata: None
        })
        .into_diff(Range { start: 2, end: 5 }),
        Some(NodeContent::Diff(DiffContent {
            content: Diff {
                content: vec![
                    Modification {
                        index: 2,
                        content: ModificationContent::Deletion(3)
                    },
                    Modification {
                        index: 2,
                        content: ModificationContent::TokenInsertion(vec![
                            ContentToken {
                                content: Bytes::from_static(&[4, 1, 2]),
                                metadata: None
                            },
                            ContentToken {
                                content: Bytes::from_static(&[6, 3]),
                                metadata: None
                            },
                        ])
                    }
                ],
            },
            model: None,
            metadata: None
        }))
    );
}

#[test]
fn nodecontent_split() {
    let model_content = Some(ContentModel {
        id: Ulid::new(),
        parameters: vec![],
    });
    let metadata_content = Some(HashMap::from([("key".to_string(), "value".to_string())]));
    let metadata_token_1 = Some(HashMap::from([("token".to_string(), "one".to_string())]));
    let metadata_token_2 = Some(HashMap::from([("token".to_string(), "two".to_string())]));
    let metadata_token_3 = Some(HashMap::from([("token".to_string(), "three".to_string())]));
    assert_eq!(
        NodeContent::Blank.split(0),
        Some((NodeContent::Blank, NodeContent::Blank))
    );
    assert_eq!(NodeContent::Blank.split(1), None);
    assert_eq!(
        NodeContent::Diff(DiffContent {
            content: Diff {
                content: vec![Modification {
                    index: 2,
                    content: ModificationContent::Deletion(3)
                }],
            },
            model: None,
            metadata: None
        })
        .split(0),
        None
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: Bytes::new(),
            model: None,
            metadata: None,
        })
        .split(0),
        Some((NodeContent::Blank, NodeContent::Blank))
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: Bytes::new(),
            model: model_content.clone(),
            metadata: metadata_content.clone(),
        })
        .split(0),
        Some((
            NodeContent::Snippet(SnippetContent {
                content: Bytes::new(),
                model: model_content.clone(),
                metadata: metadata_content.clone(),
            }),
            NodeContent::Snippet(SnippetContent {
                content: Bytes::new(),
                model: model_content.clone(),
                metadata: metadata_content.clone(),
            })
        ))
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: Bytes::from_static(&[5, 6, 7, 8]),
            model: None,
            metadata: None
        })
        .split(0),
        Some((
            NodeContent::Blank,
            NodeContent::Snippet(SnippetContent {
                content: Bytes::from_static(&[5, 6, 7, 8]),
                model: None,
                metadata: None
            })
        ))
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: Bytes::from_static(&[5, 6, 7, 8]),
            model: model_content.clone(),
            metadata: metadata_content.clone(),
        })
        .split(2),
        Some((
            NodeContent::Snippet(SnippetContent {
                content: Bytes::from_static(&[5, 6]),
                model: model_content.clone(),
                metadata: metadata_content.clone(),
            }),
            NodeContent::Snippet(SnippetContent {
                content: Bytes::from_static(&[7, 8]),
                model: model_content.clone(),
                metadata: metadata_content.clone(),
            })
        ))
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: Bytes::from_static(&[5, 6, 7, 8]),
            model: None,
            metadata: None,
        })
        .split(4),
        Some((
            NodeContent::Snippet(SnippetContent {
                content: Bytes::from_static(&[5, 6, 7, 8]),
                model: None,
                metadata: None,
            }),
            NodeContent::Blank
        ))
    );
    assert_eq!(
        NodeContent::Snippet(SnippetContent {
            content: Bytes::from_static(&[5, 6, 7, 8]),
            model: None,
            metadata: None,
        })
        .split(5),
        None
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![ContentToken {
                content: Bytes::new(),
                metadata: None
            }],
            model: None,
            metadata: None
        })
        .split(0),
        Some((NodeContent::Blank, NodeContent::Blank))
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![
                ContentToken {
                    content: Bytes::from_static(&[1, 2]),
                    metadata: metadata_token_1.clone(),
                },
                ContentToken {
                    content: Bytes::from_static(&[3, 4]),
                    metadata: metadata_token_2.clone(),
                },
                ContentToken {
                    content: Bytes::from_static(&[5, 6]),
                    metadata: metadata_token_3.clone(),
                }
            ],
            model: model_content.clone(),
            metadata: metadata_content.clone(),
        })
        .split(3),
        Some((
            NodeContent::Tokens(TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[1, 2]),
                        metadata: metadata_token_1.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[3]),
                        metadata: metadata_token_2.clone(),
                    }
                ],
                model: model_content.clone(),
                metadata: metadata_content.clone(),
            }),
            NodeContent::Tokens(TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[4]),
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[5, 6]),
                        metadata: metadata_token_3.clone(),
                    }
                ],
                model: model_content.clone(),
                metadata: metadata_content.clone(),
            })
        ))
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![
                ContentToken {
                    content: Bytes::from_static(&[1, 2]),
                    metadata: metadata_token_1.clone(),
                },
                ContentToken {
                    content: Bytes::from_static(&[3, 4]),
                    metadata: metadata_token_2.clone(),
                },
                ContentToken {
                    content: Bytes::from_static(&[5, 6]),
                    metadata: metadata_token_3.clone(),
                }
            ],
            model: model_content.clone(),
            metadata: metadata_content.clone(),
        })
        .split(2),
        Some((
            NodeContent::Tokens(TokenContent {
                content: vec![ContentToken {
                    content: Bytes::from_static(&[1, 2]),
                    metadata: metadata_token_1.clone(),
                }],
                model: model_content.clone(),
                metadata: metadata_content.clone(),
            }),
            NodeContent::Tokens(TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[3, 4]),
                        metadata: metadata_token_2.clone(),
                    },
                    ContentToken {
                        content: Bytes::from_static(&[5, 6]),
                        metadata: metadata_token_3.clone(),
                    }
                ],
                model: model_content.clone(),
                metadata: metadata_content.clone(),
            })
        ))
    );
    assert_eq!(
        NodeContent::Tokens(TokenContent {
            content: vec![
                ContentToken {
                    content: Bytes::from_static(&[1, 2]),
                    metadata: None,
                },
                ContentToken {
                    content: Bytes::from_static(&[3, 4]),
                    metadata: None,
                },
                ContentToken {
                    content: Bytes::from_static(&[5, 6]),
                    metadata: None,
                }
            ],
            model: model_content.clone(),
            metadata: metadata_content.clone(),
        })
        .split(2),
        Some((
            NodeContent::Snippet(SnippetContent {
                content: Bytes::from_static(&[1, 2]),
                model: model_content.clone(),
                metadata: metadata_content.clone(),
            }),
            NodeContent::Tokens(TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[3, 4]),
                        metadata: None,
                    },
                    ContentToken {
                        content: Bytes::from_static(&[5, 6]),
                        metadata: None,
                    }
                ],
                model: model_content.clone(),
                metadata: metadata_content.clone(),
            })
        ))
    );
}

#[test]
fn nodecontent_merge() {
    let model_content = Some(ContentModel {
        id: Ulid::new(),
        parameters: vec![],
    });
    let metadata_content = Some(HashMap::from([("key".to_string(), "value".to_string())]));
    let metadata_token_1 = Some(HashMap::from([("token".to_string(), "one".to_string())]));
    let metadata_token_2 = Some(HashMap::from([("token".to_string(), "two".to_string())]));
    let metadata_token_3 = Some(HashMap::from([("token".to_string(), "three".to_string())]));
    assert_eq!(
        NodeContent::merge(NodeContent::Blank, NodeContent::Blank),
        Some(NodeContent::Blank)
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Diff(DiffContent {
                content: Diff::default(),
                model: None,
                metadata: None
            }),
            NodeContent::Blank
        ),
        None
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Blank,
            NodeContent::Diff(DiffContent {
                content: Diff::default(),
                model: None,
                metadata: None
            })
        ),
        None
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Snippet(SnippetContent {
                content: Bytes::new(),
                model: None,
                metadata: None
            }),
            NodeContent::Snippet(SnippetContent {
                content: Bytes::new(),
                model: None,
                metadata: None
            }),
        ),
        Some(NodeContent::Blank)
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Snippet(SnippetContent {
                content: Bytes::new(),
                model: model_content.clone(),
                metadata: None
            }),
            NodeContent::Snippet(SnippetContent {
                content: Bytes::new(),
                model: None,
                metadata: None
            }),
        ),
        None
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Snippet(SnippetContent {
                content: Bytes::new(),
                model: None,
                metadata: None
            }),
            NodeContent::Snippet(SnippetContent {
                content: Bytes::new(),
                model: None,
                metadata: metadata_content.clone(),
            }),
        ),
        None
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Snippet(SnippetContent {
                content: Bytes::from_static(&[1, 2, 3]),
                model: None,
                metadata: None
            }),
            NodeContent::Blank
        ),
        Some(NodeContent::Snippet(SnippetContent {
            content: Bytes::from_static(&[1, 2, 3]),
            model: None,
            metadata: None
        }))
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Tokens(TokenContent {
                content: vec![ContentToken {
                    content: Bytes::from_static(&[1, 2, 3]),
                    metadata: metadata_token_1.clone()
                }],
                model: None,
                metadata: None
            }),
            NodeContent::Blank
        ),
        Some(NodeContent::Tokens(TokenContent {
            content: vec![ContentToken {
                content: Bytes::from_static(&[1, 2, 3]),
                metadata: metadata_token_1.clone()
            }],
            model: None,
            metadata: None
        }))
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Blank,
            NodeContent::Snippet(SnippetContent {
                content: Bytes::from_static(&[1, 2, 3]),
                model: None,
                metadata: None
            })
        ),
        Some(NodeContent::Snippet(SnippetContent {
            content: Bytes::from_static(&[1, 2, 3]),
            model: None,
            metadata: None
        }))
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Blank,
            NodeContent::Tokens(TokenContent {
                content: vec![ContentToken {
                    content: Bytes::from_static(&[1, 2, 3]),
                    metadata: metadata_token_1.clone()
                }],
                model: None,
                metadata: None
            })
        ),
        Some(NodeContent::Tokens(TokenContent {
            content: vec![ContentToken {
                content: Bytes::from_static(&[1, 2, 3]),
                metadata: metadata_token_1.clone()
            }],
            model: None,
            metadata: None
        }))
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Snippet(SnippetContent {
                content: Bytes::from_static(&[1, 2, 3]),
                model: model_content.clone(),
                metadata: metadata_content.clone()
            }),
            NodeContent::Snippet(SnippetContent {
                content: Bytes::from_static(&[4, 5, 6]),
                model: model_content.clone(),
                metadata: metadata_content.clone()
            })
        ),
        Some(NodeContent::Snippet(SnippetContent {
            content: Bytes::from_static(&[1, 2, 3, 4, 5, 6]),
            model: model_content.clone(),
            metadata: metadata_content.clone()
        }))
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Tokens(TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[1, 2, 3]),
                        metadata: metadata_token_1.clone()
                    },
                    ContentToken {
                        content: Bytes::from_static(&[4, 5]),
                        metadata: metadata_token_2.clone()
                    }
                ],
                model: model_content.clone(),
                metadata: metadata_content.clone()
            }),
            NodeContent::Snippet(SnippetContent {
                content: Bytes::from_static(&[6, 7, 8]),
                model: model_content.clone(),
                metadata: metadata_content.clone()
            })
        ),
        Some(NodeContent::Tokens(TokenContent {
            content: vec![
                ContentToken {
                    content: Bytes::from_static(&[1, 2, 3]),
                    metadata: metadata_token_1.clone()
                },
                ContentToken {
                    content: Bytes::from_static(&[4, 5]),
                    metadata: metadata_token_2.clone()
                },
                ContentToken {
                    content: Bytes::from_static(&[6, 7, 8]),
                    metadata: None
                }
            ],
            model: model_content.clone(),
            metadata: metadata_content.clone()
        }))
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Snippet(SnippetContent {
                content: Bytes::from_static(&[6, 7, 8]),
                model: model_content.clone(),
                metadata: metadata_content.clone()
            }),
            NodeContent::Tokens(TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[1, 2, 3]),
                        metadata: metadata_token_1.clone()
                    },
                    ContentToken {
                        content: Bytes::from_static(&[4, 5]),
                        metadata: metadata_token_2.clone()
                    }
                ],
                model: model_content.clone(),
                metadata: metadata_content.clone()
            })
        ),
        Some(NodeContent::Tokens(TokenContent {
            content: vec![
                ContentToken {
                    content: Bytes::from_static(&[6, 7, 8]),
                    metadata: None
                },
                ContentToken {
                    content: Bytes::from_static(&[1, 2, 3]),
                    metadata: metadata_token_1.clone()
                },
                ContentToken {
                    content: Bytes::from_static(&[4, 5]),
                    metadata: metadata_token_2.clone()
                }
            ],
            model: model_content.clone(),
            metadata: metadata_content.clone()
        }))
    );
    assert_eq!(
        NodeContent::merge(
            NodeContent::Tokens(TokenContent {
                content: vec![
                    ContentToken {
                        content: Bytes::from_static(&[1, 2, 3]),
                        metadata: metadata_token_1.clone()
                    },
                    ContentToken {
                        content: Bytes::from_static(&[4, 5]),
                        metadata: metadata_token_2.clone()
                    }
                ],
                model: model_content.clone(),
                metadata: metadata_content.clone()
            }),
            NodeContent::Tokens(TokenContent {
                content: vec![ContentToken {
                    content: Bytes::from_static(&[6, 7, 8]),
                    metadata: metadata_token_3.clone()
                },],
                model: model_content.clone(),
                metadata: metadata_content.clone()
            })
        ),
        Some(NodeContent::Tokens(TokenContent {
            content: vec![
                ContentToken {
                    content: Bytes::from_static(&[1, 2, 3]),
                    metadata: metadata_token_1.clone()
                },
                ContentToken {
                    content: Bytes::from_static(&[4, 5]),
                    metadata: metadata_token_2.clone()
                },
                ContentToken {
                    content: Bytes::from_static(&[6, 7, 8]),
                    metadata: metadata_token_3.clone()
                },
            ],
            model: model_content.clone(),
            metadata: metadata_content.clone()
        }))
    );
}

#[test]
fn diff_new() {
    assert_eq!(
        Diff::new(
            &Bytes::new(),
            &Bytes::new(),
            Instant::now() + Duration::from_secs(60)
        ),
        Diff::default()
    );
    assert_eq!(
        Diff::new(
            &Bytes::from_static(&[1, 5, 2, 2, 3, 1, 4, 1]),
            &Bytes::from_static(&[1, 5, 2, 2, 3, 1, 4, 1]),
            Instant::now() + Duration::from_secs(60)
        ),
        Diff::default()
    );
    assert_eq!(
        Diff::new(
            &Bytes::from_static(&[1, 1, 1, 1, 1, 1, 1, 1]),
            &Bytes::from_static(&[1, 1, 1, 1]),
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
            &Bytes::from_static(&[1, 1, 1, 1, 1, 1, 1, 1]),
            &Bytes::from_static(&[1, 1, 2, 2, 2, 2, 1, 1]),
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
                    content: ModificationContent::Insertion(Bytes::from_static(&[2, 2, 2, 2]))
                }
            ],
        }
    );
    assert_eq!(
        Diff::new(
            &Bytes::from_static(&[1, 1, 1, 1, 1, 1, 1, 1]),
            &Bytes::from_static(&[1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 1, 1]),
            Instant::now() + Duration::from_secs(60)
        ),
        Diff {
            content: vec![Modification {
                index: 6,
                content: ModificationContent::Insertion(Bytes::from_static(&[2, 2, 2, 2]))
            }],
        }
    );
}

/*



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
    Modification {
        index: 5,
        content: ModificationContent::Deletion(0),
    }
    .apply(&mut content);
    assert_eq!(content, vec![1, 1, 1, 1, 1, 1, 1, 1]);
    Modification {
        index: 5,
        content: ModificationContent::Insertion(vec![]),
    }
    .apply(&mut content);
    assert_eq!(content, vec![1, 1, 1, 1, 1, 1, 1, 1]);
}


*/

#[test]
#[should_panic]
fn apply_modification_out_bounds_insertion() {
    let mut content = BytesMut::from(Bytes::from_static(&[1, 1, 1, 1, 1, 1, 1, 1]));
    Modification {
        index: 9,
        content: ModificationContent::Insertion(Bytes::from_static(&[5])),
    }
    .apply(&mut content);
}

#[test]
#[should_panic]
fn apply_modification_out_bounds_deletion() {
    let mut content = BytesMut::from(Bytes::from_static(&[1, 1, 1, 1, 1, 1, 1, 1]));
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

/*

// TODO: Need to add position-change specific tests for apply_annotations

#[test]
fn modification_range_apply_annotations() {
    let mut annotations: Vec<ContentAnnotation> = vec![];

    assert_eq!(
        ModificationRange::Insertion(Range { start: 0, end: 4 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(0),
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![ContentAnnotation {
            range: Range { start: 0, end: 4 },
            metadata: None
        }]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 0, end: 4 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert!(annotations.is_empty());
    assert_eq!(
        ModificationRange::Insertion(Range { start: 0, end: 4 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(0),
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![ContentAnnotation {
            range: Range { start: 0, end: 4 },
            metadata: None
        }]
    );
    assert_eq!(
        ModificationRange::Insertion(Range { start: 4, end: 7 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(1),
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 4, end: 7 },
                metadata: None
            }
        ]
    );
    assert_eq!(
        ModificationRange::Insertion(Range { start: 4, end: 4 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 4, end: 7 },
                metadata: None
            }
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 4, end: 4 },
            tokens: vec![(0, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 4, end: 7 },
                metadata: None
            }
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 4, end: 4 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 4, end: 7 },
                metadata: None
            }
        ]
    );
    assert_eq!(
        ModificationRange::Insertion(Range { start: 7, end: 11 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(2),
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 4, end: 7 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 7, end: 11 },
                metadata: None
            }
        ]
    );
    assert_eq!(
        ModificationRange::Insertion(Range { start: 5, end: 12 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(2),
            inserted_tokens: None,
            left_split: Some(1),
            right_split: Some(3),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 4, end: 5 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 5, end: 12 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 12, end: 14 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 14, end: 18 },
                metadata: None
            }
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 12, end: 14 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 4, end: 5 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 5, end: 12 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 12, end: 16 },
                metadata: None
            }
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 6, end: 11 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: Some(2),
            right_split: Some(3),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 4, end: 5 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 5, end: 6 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 6, end: 7 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 7, end: 11 },
                metadata: None
            }
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 3, end: 8 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: Some(0),
            right_split: Some(1),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 3 },
                metadata: None
            },
            ContentAnnotation {
                range: Range { start: 3, end: 6 },
                metadata: None
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 0, end: 6 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert!(annotations.is_empty());
    annotations = vec![
        ContentAnnotation {
            range: Range { start: 0, end: 4 },
            metadata: None,
        },
        ContentAnnotation {
            range: Range { start: 4, end: 6 },
            metadata: None,
        },
        ContentAnnotation {
            range: Range { start: 6, end: 11 },
            metadata: None,
        },
        ContentAnnotation {
            range: Range { start: 11, end: 15 },
            metadata: None,
        },
    ];
    assert_eq!(
        ModificationRange::Insertion(Range { start: 0, end: 8 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(0),
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    annotations = vec![
        ContentAnnotation {
            range: Range { start: 0, end: 8 },
            metadata: None,
        },
        ContentAnnotation {
            range: Range { start: 8, end: 12 },
            metadata: None,
        },
        ContentAnnotation {
            range: Range { start: 12, end: 14 },
            metadata: None,
        },
        ContentAnnotation {
            range: Range { start: 14, end: 19 },
            metadata: None,
        },
        ContentAnnotation {
            range: Range { start: 19, end: 23 },
            metadata: None,
        },
    ];
    assert_eq!(
        ModificationRange::Deletion(Range { start: 0, end: 8 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 11 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 11, end: 15 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Insertion(Range { start: 4, end: 12 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(1),
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 12 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 12, end: 14 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 14, end: 19 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 19, end: 23 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Insertion(Range { start: 14, end: 16 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(3),
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 12 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 12, end: 14 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 14, end: 16 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 16, end: 21 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 21, end: 25 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 4, end: 12 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 8 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 8, end: 13 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 13, end: 17 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Insertion(Range { start: 13, end: 16 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(4),
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 8 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 8, end: 13 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 13, end: 16 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 16, end: 20 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Insertion(Range { start: 20, end: 23 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(6),
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 8 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 8, end: 13 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 13, end: 16 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 16, end: 20 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 20, end: 23 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 8, end: 13 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 8 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 8, end: 11 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 11, end: 15 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 15, end: 18 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 11, end: 15 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 8 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 8, end: 11 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 11, end: 14 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 11, end: 14 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 8 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 8, end: 11 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Insertion(Range { start: 2, end: 4 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(1),
            inserted_tokens: None,
            left_split: Some(0),
            right_split: Some(2),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 8 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 8, end: 10 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 10, end: 13 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 1, end: 4 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: Some(0),
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 10 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 1, end: 4 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: Some(1),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 7 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Insertion(Range { start: 3, end: 4 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(3),
            inserted_tokens: None,
            left_split: Some(2),
            right_split: Some(4),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 8 },
                metadata: None,
            }
        ]
    );
    assert_eq!(
        ModificationRange::Insertion(Range { start: 6, end: 9 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: Some(6),
            inserted_tokens: None,
            left_split: Some(5),
            right_split: Some(7),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 11 },
                metadata: None,
            }
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 0, end: 6 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 5 },
                metadata: None,
            }
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 1, end: 6 },
            tokens: vec![(2, None), (3, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(1..=2),
            left_split: Some(0),
            right_split: Some(3),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 8 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 8, end: 10 },
                metadata: None,
            }
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 6, end: 9 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: Some(3),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 7 },
                metadata: None,
            }
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 5, end: 7 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: Some(2),
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 5 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 0, end: 5 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert!(annotations.is_empty());
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 0, end: 9 },
            tokens: vec![(4, None), (3, None), (2, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(0..=2),
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 9 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 7, end: 9 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 7 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 0, end: 6 },
            tokens: vec![(2, None), (4, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(0..=1),
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 10 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 10, end: 13 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 11, end: 13 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: Some(3),
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 10 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 10, end: 11 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 7, end: 10 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: Some(2),
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 8 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 6, end: 8 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 6 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 2, end: 8 },
            tokens: vec![(3, None), (3, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(1..=2),
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 8 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 8, end: 12 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 9, end: 11 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: Some(3),
            right_split: Some(4),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 8 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 8, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 10 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 0, end: 2 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 8 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 1, end: 2 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: Some(0),
            right_split: Some(1),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 7 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 7, end: 12 },
            tokens: vec![(2, None), (3, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(5..=6),
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 12 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 0, end: 2 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 10 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 7, end: 12 },
            tokens: vec![(2, None), (3, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(4..=5),
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 12 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 12, end: 15 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 6, end: 11 },
            tokens: vec![(2, None), (3, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(4..=5),
            left_split: Some(3),
            right_split: Some(6),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 8 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 8, end: 11 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 11, end: 12 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 12, end: 14 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 14, end: 17 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 17, end: 20 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 4, end: 12 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 12 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 11, end: 16 },
            tokens: vec![(2, None), (3, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(5..=6),
            left_split: Some(4),
            right_split: Some(7),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 6 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 6, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 11 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 11, end: 13 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 13, end: 16 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 16, end: 17 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 3, end: 9 }).apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 10 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 10, end: 11 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 0, end: 2 },
            tokens: vec![(2, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(0..=0),
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 12 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 12, end: 13 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 13, end: 15 },
            tokens: vec![(2, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(6..=6),
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 12 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 12, end: 13 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 13, end: 15 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::Deletion(Range { start: 7, end: 13 })
            .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: None,
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 9 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 5, end: 7 },
            tokens: vec![(2, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(2..=2),
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 2 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 2, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 11 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 1, end: 3 },
            tokens: vec![(2, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(1..=1),
            left_split: Some(0),
            right_split: Some(2),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 11 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 11, end: 13 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 5, end: 7 },
            tokens: vec![(2, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(4..=4),
            left_split: Some(3),
            right_split: Some(5),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 11 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 11, end: 13 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 13, end: 15 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 14, end: 16 },
            tokens: vec![(2, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(9..=9),
            left_split: Some(8),
            right_split: Some(10),
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 11 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 11, end: 13 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 13, end: 14 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 14, end: 16 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 16, end: 17 },
                metadata: None,
            },
        ]
    );
    assert_eq!(
        ModificationRange::TokenInsertion(ModificationRangeTokens {
            range: Range { start: 17, end: 19 },
            tokens: vec![(2, None)]
        })
        .apply_annotations(&mut annotations),
        ModificationIndices {
            inserted_bytes: None,
            inserted_tokens: Some(11..=11),
            left_split: None,
            right_split: None,
        }
    );
    assert_eq!(
        annotations,
        vec![
            ContentAnnotation {
                range: Range { start: 0, end: 1 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 1, end: 3 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 3, end: 4 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 4, end: 5 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 5, end: 7 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 7, end: 9 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 9, end: 11 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 11, end: 13 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 13, end: 14 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 14, end: 16 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 16, end: 17 },
                metadata: None,
            },
            ContentAnnotation {
                range: Range { start: 17, end: 19 },
                metadata: None,
            },
        ]
    );
}

     */

#[test]
#[should_panic]
fn modification_range_invalid_token_insertion_over_length() {
    let mut annotations: LinkedList<ContentAnnotation> = LinkedList::new();

    let mut location = 0;
    let mut cursor = annotations.cursor_front_mut();

    ModificationRange::TokenInsertion(ModificationRangeTokens {
        range: Range { start: 0, end: 6 },
        tokens: vec![(2, None), (3, None)],
    })
    .apply_annotations(
        &mut location,
        &mut cursor,
        |_| {},
        |_, _| {},
        |_| {},
        |_| {},
    );
}

#[test]
#[should_panic]
fn modification_range_invalid_token_insertion_under_length() {
    let mut annotations: LinkedList<ContentAnnotation> = LinkedList::new();

    let mut location = 0;
    let mut cursor = annotations.cursor_front_mut();

    ModificationRange::TokenInsertion(ModificationRangeTokens {
        range: Range { start: 0, end: 4 },
        tokens: vec![(2, None), (3, None)],
    })
    .apply_annotations(
        &mut location,
        &mut cursor,
        |_| {},
        |_, _| {},
        |_| {},
        |_| {},
    );
}

#[test]
#[should_panic]
fn modification_range_invalid_insert_modification_index() {
    let mut annotations: LinkedList<ContentAnnotation> = LinkedList::from([ContentAnnotation {
        len: 4,
        metadata: None,
    }]);

    let mut location = 0;
    let mut cursor = annotations.cursor_front_mut();

    ModificationRange::Insertion(Range { start: 5, end: 6 }).apply_annotations(
        &mut location,
        &mut cursor,
        |_| {},
        |_, _| {},
        |_| {},
        |_| {},
    );
}

#[test]
#[should_panic]
fn modification_range_invalid_token_insert_modification_index() {
    let mut annotations: LinkedList<ContentAnnotation> = LinkedList::from([ContentAnnotation {
        len: 4,
        metadata: None,
    }]);

    let mut location = 0;
    let mut cursor = annotations.cursor_front_mut();

    ModificationRange::TokenInsertion(ModificationRangeTokens {
        range: Range { start: 5, end: 10 },
        tokens: vec![(2, None), (3, None)],
    })
    .apply_annotations(
        &mut location,
        &mut cursor,
        |_| {},
        |_, _| {},
        |_| {},
        |_| {},
    );
}

#[test]
#[should_panic]
fn modification_range_invalid_deletion_modification_index() {
    let mut annotations: LinkedList<ContentAnnotation> = LinkedList::from([ContentAnnotation {
        len: 4,
        metadata: None,
    }]);

    let mut location = 0;
    let mut cursor = annotations.cursor_front_mut();

    ModificationRange::Deletion(Range { start: 3, end: 6 }).apply_annotations(
        &mut location,
        &mut cursor,
        |_| {},
        |_, _| {},
        |_| {},
        |_| {},
    );
}

#[test]
fn weave_timeline_annotated_string_valid_utf8() {
    let nodes = [
        Node {
            id: Ulid::from_parts(0, 0),
            from: HashSet::new(),
            to: HashSet::from([Ulid::from_parts(0, 1)]),
            active: true,
            bookmarked: false,
            content: NodeContent::Snippet(SnippetContent {
                content: "I wish you a happy".into(),
                model: None,
                metadata: Some(HashMap::from([("index".to_string(), "0".to_string())])),
            }),
        },
        Node {
            id: Ulid::from_parts(0, 1),
            from: HashSet::from([Ulid::from_parts(0, 0)]),
            to: HashSet::from([Ulid::from_parts(0, 2)]),
            active: true,
            bookmarked: false,
            content: NodeContent::Tokens(TokenContent {
                content: vec![
                    ContentToken {
                        content: " new".into(),
                        metadata: Some(HashMap::from([(
                            "token_index".to_string(),
                            "0".to_string(),
                        )])),
                    },
                    ContentToken {
                        content: " year!".into(),
                        metadata: Some(HashMap::from([(
                            "token_index".to_string(),
                            "1".to_string(),
                        )])),
                    },
                ],
                model: Some(ContentModel {
                    id: Ulid::from_parts(99, 0),
                    parameters: vec![("parameter".to_string(), "value".to_string())],
                }),
                metadata: Some(HashMap::from([("index".to_string(), "1".to_string())])),
            }),
        },
        Node {
            id: Ulid::from_parts(0, 2),
            from: HashSet::from([Ulid::from_parts(0, 1)]),
            to: HashSet::from([Ulid::from_parts(0, 3)]),
            active: true,
            bookmarked: false,
            content: NodeContent::Diff(DiffContent {
                content: Diff {
                    content: vec![
                        Modification {
                            index: 21,
                            content: ModificationContent::Deletion(1),
                        },
                        Modification {
                            index: 21,
                            content: ModificationContent::Insertion("xt".into()),
                        },
                        Modification {
                            index: 28,
                            content: ModificationContent::Deletion(1),
                        },
                    ],
                },
                model: None,
                metadata: Some(HashMap::from([("index".to_string(), "2".to_string())])),
            }),
        },
        Node {
            id: Ulid::from_parts(0, 3),
            from: HashSet::from([Ulid::from_parts(0, 2)]),
            to: HashSet::from([Ulid::from_parts(0, 4)]),
            active: true,
            bookmarked: false,
            content: NodeContent::Snippet(SnippetContent {
                content: " my friend!".into(),
                model: None,
                metadata: Some(HashMap::from([("index".to_string(), "3".to_string())])),
            }),
        },
        Node {
            id: Ulid::from_parts(0, 4),
            from: HashSet::from([Ulid::from_parts(0, 3)]),
            to: HashSet::new(),
            active: true,
            bookmarked: false,
            content: NodeContent::Blank,
        },
    ];
    let models = [Model {
        id: Ulid::from_parts(99, 0),
        label: "Test Model".to_string(),
        metadata: HashMap::new(),
    }];
    let timeline = WeaveTimeline {
        timeline: vec![
            (&nodes[0], None),
            (&nodes[1], Some(&models[0])),
            (&nodes[2], None),
            (&nodes[3], None),
            (&nodes[4], None),
        ],
    };
    let (string, annotations) = timeline.annotated_string();
    assert_eq!(
        string,
        "I wish you a happy next year my friend!".to_string()
    );
    assert_eq!(
        annotations,
        LinkedList::from([
            TimelineAnnotation {
                len: 18,
                node: Some(&nodes[0]),
                model: None,
                parameters: None,
                subsection_metadata: None,
                content_metadata: nodes[0].content.metadata(),
            },
            TimelineAnnotation {
                len: 3,
                node: Some(&nodes[1]),
                model: Some(&models[0]),
                parameters: nodes[1].content.model().map(|model| &model.parameters),
                subsection_metadata: Some(&HashMap::from([(
                    "token_index".to_string(),
                    "0".to_string(),
                )])),
                content_metadata: nodes[1].content.metadata(),
            },
            TimelineAnnotation {
                len: 2,
                node: Some(&nodes[2]),
                model: None,
                parameters: None,
                subsection_metadata: None,
                content_metadata: nodes[2].content.metadata(),
            },
            TimelineAnnotation {
                len: 5,
                node: Some(&nodes[1]),
                model: Some(&models[0]),
                parameters: nodes[1].content.model().map(|model| &model.parameters),
                subsection_metadata: Some(&HashMap::from([(
                    "token_index".to_string(),
                    "1".to_string(),
                )])),
                content_metadata: nodes[1].content.metadata(),
            },
            TimelineAnnotation {
                len: 11,
                node: Some(&nodes[3]),
                model: None,
                parameters: None,
                subsection_metadata: None,
                content_metadata: nodes[3].content.metadata(),
            },
        ])
    );
}

#[test]
fn weave_timeline_annotated_string_invalid_utf8() {
    let nodes = [
        Node {
            id: Ulid::from_parts(0, 0),
            from: HashSet::new(),
            to: HashSet::from([Ulid::from_parts(0, 1)]),
            active: true,
            bookmarked: false,
            content: NodeContent::Snippet(SnippetContent {
                content: "I love you".into(),
                model: None,
                metadata: Some(HashMap::from([("index".to_string(), "0".to_string())])),
            }),
        },
        Node {
            id: Ulid::from_parts(0, 1),
            from: HashSet::from([Ulid::from_parts(0, 0)]),
            to: HashSet::from([Ulid::from_parts(0, 2)]),
            active: true,
            bookmarked: false,
            content: NodeContent::Tokens(TokenContent {
                content: vec![
                    ContentToken {
                        content: " so".into(),
                        metadata: Some(HashMap::from([(
                            "token_index".to_string(),
                            "0".to_string(),
                        )])),
                    },
                    ContentToken {
                        content: " much!".into(),
                        metadata: Some(HashMap::from([(
                            "token_index".to_string(),
                            "1".to_string(),
                        )])),
                    },
                ],
                model: Some(ContentModel {
                    id: Ulid::from_parts(99, 0),
                    parameters: vec![("content_model_index".to_string(), "0".to_string())],
                }),
                metadata: Some(HashMap::from([("index".to_string(), "1".to_string())])),
            }),
        },
        Node {
            id: Ulid::from_parts(0, 2),
            from: HashSet::from([Ulid::from_parts(0, 1)]),
            to: HashSet::from([Ulid::from_parts(0, 3)]),
            active: true,
            bookmarked: false,
            content: NodeContent::Diff(DiffContent {
                content: Diff {
                    content: vec![
                        Modification {
                            index: 10,
                            content: ModificationContent::Deletion(9),
                        },
                        Modification {
                            index: 10,
                            content: ModificationContent::TokenInsertion(vec![
                                ContentToken {
                                    content: "~ ".into(),
                                    metadata: Some(HashMap::from([(
                                        "token_index".to_string(),
                                        "3".to_string(),
                                    )])),
                                },
                                ContentToken {
                                    content: "💖".as_bytes()[..3].into(),
                                    metadata: Some(HashMap::from([(
                                        "token_index".to_string(),
                                        "4".to_string(),
                                    )])),
                                },
                            ]),
                        },
                    ],
                },
                model: Some(ContentModel {
                    id: Ulid::from_parts(99, 0),
                    parameters: vec![("content_model_index".to_string(), "1".to_string())],
                }),
                metadata: Some(HashMap::from([("index".to_string(), "2".to_string())])),
            }),
        },
        Node {
            id: Ulid::from_parts(0, 3),
            from: HashSet::from([Ulid::from_parts(0, 2)]),
            to: HashSet::from([Ulid::from_parts(0, 4)]),
            active: true,
            bookmarked: false,
            content: NodeContent::Snippet(SnippetContent {
                content: "\nMwah! <3".into(),
                model: None,
                metadata: Some(HashMap::from([("index".to_string(), "3".to_string())])),
            }),
        },
    ];
    let models = [Model {
        id: Ulid::from_parts(99, 0),
        label: "Test Model".to_string(),
        metadata: HashMap::new(),
    }];
    let timeline = WeaveTimeline {
        timeline: vec![
            (&nodes[0], None),
            (&nodes[1], Some(&models[0])),
            (&nodes[2], Some(&models[0])),
            (&nodes[3], None),
        ],
    };
    let (string, annotations) = timeline.annotated_string();
    assert_eq!(string, "I love you~ \nMwah! <3".to_string());
    assert_eq!(
        annotations,
        LinkedList::from([
            TimelineAnnotation {
                len: 10,
                node: Some(&nodes[0]),
                model: None,
                parameters: None,
                subsection_metadata: None,
                content_metadata: nodes[0].content.metadata(),
            },
            TimelineAnnotation {
                len: 2,
                node: Some(&nodes[2]),
                model: Some(&models[0]),
                parameters: nodes[2].content.model().map(|model| &model.parameters),
                subsection_metadata: Some(&HashMap::from([(
                    "token_index".to_string(),
                    "3".to_string(),
                )])),
                content_metadata: nodes[2].content.metadata(),
            },
            TimelineAnnotation {
                len: 3,
                node: Some(&nodes[2]),
                model: Some(&models[0]),
                parameters: nodes[2].content.model().map(|model| &model.parameters),
                subsection_metadata: Some(&HashMap::from([(
                    "token_index".to_string(),
                    "4".to_string(),
                )])),
                content_metadata: nodes[2].content.metadata(),
            },
            TimelineAnnotation {
                len: 9,
                node: Some(&nodes[3]),
                model: None,
                parameters: None,
                subsection_metadata: None,
                content_metadata: nodes[3].content.metadata(),
            },
        ])
    );
}
