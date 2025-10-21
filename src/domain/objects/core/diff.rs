use derive_new::new;
use std::fmt::Display;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Edit<T> {
    Delete { value: T },
    Insert { value: T },
    Equal { value: T },
}

impl<T> Edit<T>
where
    T: Clone + Into<String>,
{
    pub fn as_string(&self) -> String {
        match self {
            Edit::Delete { value } => format!("-{}", value.clone().into()),
            Edit::Insert { value } => format!("+{}", value.clone().into()),
            Edit::Equal { value } => format!(" {}", value.clone().into()),
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

pub trait DiffAlgorithm<'d, T> {
    type Trace;
    type EditPath;
    type EditScript;
    type Output;

    fn compute_shortest_edit(&self) -> Self::Trace;
    fn backtrack(&self) -> Self::EditPath;
    fn diff(&self) -> Self::EditScript;
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

#[derive(Debug, Clone, PartialEq, Eq, new)]
pub struct MyersDiff<'d, T> {
    a: &'d [T],
    b: &'d [T],
}

impl<'d, T: Eq + Clone> DiffAlgorithm<'d, T> for MyersDiff<'d, T> {
    type Trace = Vec<Vec<isize>>;
    type EditPath = Vec<(isize, isize, isize, isize)>;
    type EditScript = Vec<Edit<T>>;
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
                while x < n && y < m && self.a[x as usize] == self.b[y as usize] {
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
                    diff.push(Edit::Insert {
                        value: self.b[prev_y as usize].clone(),
                    });
                }
            } else if y == prev_y {
                // Delete: only x increased
                if prev_x < self.a.len() as isize {
                    diff.push(Edit::Delete {
                        value: self.a[prev_x as usize].clone(),
                    });
                }
            } else {
                // Equal: both increased (diagonal move)
                if prev_x < self.a.len() as isize {
                    diff.push(Edit::Equal {
                        value: self.a[prev_x as usize].clone(),
                    });
                }
            }
        }

        diff.reverse();
        diff
    }
}

#[cfg(test)]
mod tests {
    use crate::domain::objects::diff::{DiffAlgorithm, Edit, MyersDiff};
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
            Edit::Delete { value: 'a' },
            Edit::Delete { value: 'b' },
            Edit::Equal { value: 'c' },
            Edit::Insert { value: 'b' },
            Edit::Equal { value: 'a' },
            Edit::Equal { value: 'b' },
            Edit::Delete { value: 'b' },
            Edit::Equal { value: 'a' },
            Edit::Insert { value: 'c' },
        ];

        assert_eq!(result, expected);
    }

    #[rstest]
    fn test_diff_files(file_inputs: (Vec<&'static str>, Vec<&'static str>)) {
        let (a, b) = file_inputs;
        let result = MyersDiff::new(&a, &b).diff();
        let expected = vec![
            Edit::Delete { value: "line1" },
            Edit::Equal { value: "line2" },
            Edit::Delete { value: "line3" },
            Edit::Insert {
                value: "line3_modified",
            },
            Edit::Equal { value: "line4" },
            Edit::Insert { value: "line5" },
        ];

        assert_eq!(result, expected);
    }
}
