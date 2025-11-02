use derive_new::new;
use std::fmt::Display;

type Lines<T> = Vec<Line<T>>;

#[derive(Debug, Clone, PartialEq, Eq, new)]
pub struct Line<T> {
    number: usize,
    value: T,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit<T> {
    Delete { line: Line<T> },
    Insert { line: Line<T> },
    Equal { line_a: Line<T>, line_b: Line<T> },
}

impl<T> Edit<T>
where
    T: Clone + Into<String>,
{
    pub fn as_string(&self) -> String {
        match self {
            Edit::Delete { line } => format!("-{}", line.value.clone().into()),
            Edit::Insert { line } => format!("+{}", line.value.clone().into()),
            Edit::Equal { line_a, .. } => format!(" {}", line_a.value.clone().into()),
        }
    }
}

impl<T> Edit<T> {
    pub fn line_a(&self) -> &Line<T> {
        match self {
            Edit::Delete { line } => line,
            Edit::Insert { line } => line,
            Edit::Equal { line_a, .. } => line_a,
        }
    }

    pub fn line_b(&self) -> &Line<T> {
        match self {
            Edit::Delete { line } => line,
            Edit::Insert { line } => line,
            Edit::Equal { line_b, .. } => line_b,
        }
    }
}

impl<T> Display for Edit<T>
where
    T: Clone + Into<String>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_string())
    }
}

const HUNK_CONTEXT: isize = 3;

#[derive(Debug, Clone, PartialEq, Eq, new)]
pub struct Hunk<T> {
    a_start: usize,
    b_start: usize,
    edits: Vec<Edit<T>>,
}

impl<T> Hunk<T> {
    pub fn a_start(&self) -> usize {
        self.a_start
    }

    pub fn b_start(&self) -> usize {
        self.b_start
    }

    pub fn edits(&self) -> &[Edit<T>] {
        &self.edits
    }

    pub fn a_size(&self) -> usize {
        self.edits
            .iter()
            .filter(|edit| matches!(edit, Edit::Delete { .. } | Edit::Equal { .. }))
            .count()
    }

    pub fn b_size(&self) -> usize {
        self.edits
            .iter()
            .filter(|edit| matches!(edit, Edit::Insert { .. } | Edit::Equal { .. }))
            .count()
    }
}

pub trait DiffAlgorithm<T> {
    type Trace;
    type EditPath;
    type EditScript;
    type Hunks;
    type Output;

    fn compute_shortest_edit(&self) -> Self::Trace;
    fn backtrack(&self) -> Self::EditPath;
    fn diff(&self) -> Self::EditScript;
    fn flatten_diff(&self) -> Self::Hunks;
    fn format_diff(&self) -> Self::Output
    where
        T: Clone + Into<String>,
        Self::EditScript: AsRef<[Edit<T>]>,
        Self::Output: From<String>,
    {
        let edits = self.diff();
        let formatted = edits
            .as_ref()
            .iter()
            .map(|edit| edit.as_string())
            .collect::<Vec<_>>()
            .join("\n");
        formatted.into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MyersDiff<T> {
    a: Lines<T>,
    b: Lines<T>,
}

impl<T: Eq + Clone> MyersDiff<T> {
    pub fn new(a: &[T], b: &[T]) -> Self {
        let a_lines = Self::lines(a);
        let b_lines = Self::lines(b);

        MyersDiff {
            a: a_lines,
            b: b_lines,
        }
    }

    fn lines(document: &[T]) -> Lines<T>
    where
        T: Clone,
    {
        document
            .iter()
            .enumerate()
            .map(|(i, v)| Line {
                number: i + 1,
                value: v.clone(),
            })
            .collect::<Vec<_>>()
    }
}

impl<T: Eq + Clone> DiffAlgorithm<T> for MyersDiff<T> {
    type Trace = Vec<Vec<isize>>;
    type EditPath = Vec<(isize, isize, isize, isize)>;
    type EditScript = Vec<Edit<T>>;
    type Hunks = Vec<Hunk<T>>;
    type Output = String;

    fn compute_shortest_edit(&self) -> Self::Trace {
        let (n, m) = (self.a.len() as isize, self.b.len() as isize);
        let offset = (n + m) as usize;

        let mut v = vec![0; 2 * offset + 1];
        v[offset] = 0; // v[0] = 0

        let mut trace = Vec::new();

        for d in 0..=(n + m) {
            trace.push(v.clone());

            for k in (-d..=d).step_by(2) {
                let idx = (offset as isize + k) as usize;

                let mut x = if k == -d {
                    // we could have only come from k+1, thus an insertion
                    v[idx + 1]
                } else if k == d {
                    // we could have only come from k-1, thus a deletion
                    v[idx - 1] + 1
                } else {
                    // we could have come from either k-1 (deletion) or k+1 (insertion)
                    let x_del = v[idx - 1] + 1;
                    let x_ins = v[idx + 1];
                    if x_del > x_ins { x_del } else { x_ins }
                };

                let mut y = x - k;
                while x < n && y < m && self.a[x as usize].value == self.b[y as usize].value {
                    // snake
                    x += 1;
                    y += 1;
                }

                v[idx] = x;

                if x >= n && y >= m {
                    return trace;
                }
            }
        }

        trace
    }

    fn backtrack(&self) -> Self::EditPath {
        let (mut x, mut y) = (self.a.len() as isize, self.b.len() as isize);
        let offset = (x + y) as usize;
        let mut edit_path = Vec::new();

        let trace = self.compute_shortest_edit();

        for (d, v) in trace.iter().enumerate().rev() {
            let k = x - y;

            let prev_k = if k == -(d as isize) {
                k + 1
            } else if k == (d as isize) {
                k - 1
            } else {
                let k_del = k - 1;
                let k_ins = k + 1;
                if v[(offset as isize + k_del) as usize] + 1 > v[(offset as isize + k_ins) as usize]
                {
                    k_del
                } else {
                    k_ins
                }
            };

            let prev_x = v[(offset as isize + prev_k) as usize];
            let prev_y = prev_x - prev_k;

            while x > prev_x && y > prev_y {
                edit_path.push((x - 1, y - 1, x, y));
                x -= 1;
                y -= 1;
            }

            if d > 0 {
                edit_path.push((prev_x, prev_y, x, y));
            }

            (x, y) = (prev_x, prev_y);
        }

        edit_path
    }

    fn diff(&self) -> Self::EditScript {
        let mut diff = Vec::new();

        let path = self.backtrack();

        for (prev_x, prev_y, x, y) in path {
            if x == prev_x {
                // Insert: only y increased
                if prev_y < self.b.len() as isize {
                    let line_b = self.b[prev_y as usize].clone();
                    diff.push(Edit::Insert { line: line_b });
                }
            } else if y == prev_y {
                // Delete: only x increased
                if prev_x < self.a.len() as isize {
                    let line_a = self.a[prev_x as usize].clone();
                    diff.push(Edit::Delete { line: line_a });
                }
            } else {
                // Equal: both increased (diagonal move)
                if prev_x < self.a.len() as isize {
                    let line_a = self.a[prev_x as usize].clone();
                    let line_b = self.b[prev_y as usize].clone();
                    diff.push(Edit::Equal { line_a, line_b });
                }
            }
        }

        diff.reverse();
        diff
    }

    fn flatten_diff(&self) -> Self::Hunks {
        let edits = self.diff();

        let mut hunks = Vec::new();
        let mut offset = 0_isize;

        let collect_hunk_edits = |offset: &mut isize| -> Vec<Edit<T>> {
            let mut counter = -1;

            let mut hunk_edits = Vec::new();
            while counter != 0 {
                if *offset >= 0 && counter > 0 {
                    hunk_edits.push(edits[*offset as usize].clone());
                }

                *offset += 1;
                if *offset >= edits.len() as isize {
                    break;
                }

                if *offset + HUNK_CONTEXT >= edits.len() as isize {
                    counter -= 1;
                } else {
                    match &edits[(*offset + HUNK_CONTEXT) as usize] {
                        Edit::Delete { .. } | Edit::Insert { .. } => {
                            counter = 2 * HUNK_CONTEXT + 1;
                        }
                        Edit::Equal { .. } => {
                            counter -= 1;
                        }
                    }
                }
            }

            hunk_edits
        };

        loop {
            while offset < edits.len() as isize
                && let Edit::Equal { .. } = edits[offset as usize]
            {
                offset += 1;
            }

            if offset >= edits.len() as isize {
                return hunks;
            }

            let start_offset = (offset - HUNK_CONTEXT).max(0);

            let a_start = edits[start_offset as usize].line_a().number;
            let b_start = edits[start_offset as usize].line_b().number;

            offset -= HUNK_CONTEXT + 1;

            hunks.push(Hunk::new(a_start, b_start, collect_hunk_edits(&mut offset)));
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::objects::diff::{DiffAlgorithm, Edit, Line, MyersDiff};
    use pretty_assertions::assert_eq;
    use rstest::{fixture, rstest};

    #[fixture]
    fn string_inputs() -> (Vec<char>, Vec<char>) {
        ("abcabba".chars().collect(), "cbabac".chars().collect())
    }

    #[fixture]
    fn file_inputs() -> (Vec<&'static str>, Vec<&'static str>) {
        (
            vec!["line1", "line2", "line3", "line4"],
            vec!["line2", "line3_modified", "line4", "line5"],
        )
    }

    #[rstest]
    fn test_diff_strings(string_inputs: (Vec<char>, Vec<char>)) {
        let (a, b) = string_inputs;
        let result = MyersDiff::new(&a, &b).diff();
        let expected = vec![
            Edit::Delete {
                line: Line {
                    number: 1,
                    value: 'a',
                },
            },
            Edit::Delete {
                line: Line {
                    number: 2,
                    value: 'b',
                },
            },
            Edit::Equal {
                line_a: Line {
                    number: 3,
                    value: 'c',
                },
                line_b: Line {
                    number: 1,
                    value: 'c',
                },
            },
            Edit::Insert {
                line: Line {
                    number: 2,
                    value: 'b',
                },
            },
            Edit::Equal {
                line_a: Line {
                    number: 4,
                    value: 'a',
                },
                line_b: Line {
                    number: 3,
                    value: 'a',
                },
            },
            Edit::Equal {
                line_a: Line {
                    number: 5,
                    value: 'b',
                },
                line_b: Line {
                    number: 4,
                    value: 'b',
                },
            },
            Edit::Delete {
                line: Line {
                    number: 6,
                    value: 'b',
                },
            },
            Edit::Equal {
                line_a: Line {
                    number: 7,
                    value: 'a',
                },
                line_b: Line {
                    number: 5,
                    value: 'a',
                },
            },
            Edit::Insert {
                line: Line {
                    number: 6,
                    value: 'c',
                },
            },
        ];

        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_diff_files(file_inputs: (Vec<&'static str>, Vec<&'static str>)) {
        let (a, b) = file_inputs;
        let result = MyersDiff::new(&a, &b).diff();
        let expected = vec![
            Edit::Delete {
                line: Line {
                    number: 1,
                    value: "line1",
                },
            },
            Edit::Equal {
                line_a: Line {
                    number: 2,
                    value: "line2",
                },
                line_b: Line {
                    number: 1,
                    value: "line2",
                },
            },
            Edit::Delete {
                line: Line {
                    number: 3,
                    value: "line3",
                },
            },
            Edit::Insert {
                line: Line {
                    number: 2,
                    value: "line3_modified",
                },
            },
            Edit::Equal {
                line_a: Line {
                    number: 4,
                    value: "line4",
                },
                line_b: Line {
                    number: 3,
                    value: "line4",
                },
            },
            Edit::Insert {
                line: Line {
                    number: 4,
                    value: "line5",
                },
            },
        ];

        assert_eq!(result, expected);
    }
}
