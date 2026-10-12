use super::super::LogEntry;
use std::{collections::VecDeque, ops::Range};

// Bound the work for a visible paragraph, never the original log message.
const CHUNK_CHARS: usize = 512;
pub(super) const ROW_GAP: f32 = 12.0;

#[derive(Clone)]
pub(super) struct Row {
    pub entry: usize,
    pub bytes: Range<usize>,
    pub kind: Kind,
    units: f32,
    height: f32,
}

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Debug,
    Info,
    Error,
}

#[derive(Default)]
pub(crate) struct LogView {
    rows: Vec<Row>,
    offsets: Vec<f32>,
    indexed_entries: usize,
    errors_only: bool,
    width: f32,
    pixels_per_point: f32,
}

impl LogView {
    pub(super) fn clear(&mut self) {
        *self = Self::default();
    }

    pub(super) fn sync(
        &mut self,
        logs: &VecDeque<LogEntry>,
        errors_only: bool,
        width: f32,
        pixels_per_point: f32,
    ) {
        if self.indexed_entries > logs.len() || self.errors_only != errors_only {
            self.rows.clear();
            self.offsets.clear();
            self.indexed_entries = 0;
        }
        let reflow = (self.width - width).abs() > 0.5
            || (self.pixels_per_point - pixels_per_point).abs() > 0.01;
        self.errors_only = errors_only;
        self.width = width;
        self.pixels_per_point = pixels_per_point;
        let first_new_row = self.rows.len();
        for (entry, log) in logs.iter().enumerate().skip(self.indexed_entries) {
            if errors_only && !log.failed {
                continue;
            }
            let kind = if log.failed {
                Kind::Error
            } else if ["已通过认证", "认证成功", "已确认离线"]
                .iter()
                .any(|text| log.message.contains(text))
            {
                Kind::Info
            } else {
                Kind::Debug
            };
            let mut start = 0;
            let mut chars = 0;
            let mut units = 0.0;
            for (offset, ch) in log.message.char_indices() {
                chars += 1;
                if ch != '\n' {
                    units += if ch.is_ascii() { 6.8 } else { 13.0 };
                }
                if chars == CHUNK_CHARS || ch == '\n' {
                    let end = offset + ch.len_utf8();
                    self.rows.push(Row {
                        entry,
                        bytes: start..end,
                        kind,
                        units,
                        height: estimate(units, width),
                    });
                    start = end;
                    chars = 0;
                    units = 0.0;
                }
            }
            if start < log.message.len() || log.message.is_empty() {
                self.rows.push(Row {
                    entry,
                    bytes: start..log.message.len(),
                    kind,
                    units,
                    height: estimate(units, width),
                });
            }
        }
        self.indexed_entries = logs.len();
        if reflow {
            for row in &mut self.rows {
                row.height = estimate(row.units, width);
            }
            self.rebuild_offsets(0);
        } else if first_new_row != self.rows.len() {
            self.rebuild_offsets(first_new_row);
        }
    }

    fn rebuild_offsets(&mut self, first: usize) {
        self.offsets.truncate(first + 1);
        if self.offsets.is_empty() {
            self.offsets.push(0.0);
        }
        for row in &self.rows[first..] {
            self.offsets
                .push(self.offsets.last().copied().unwrap_or(0.0) + row.height);
        }
    }

    pub(super) fn total_height(&self) -> f32 {
        self.offsets.last().copied().unwrap_or(0.0)
    }
    pub(super) fn row(&self, index: usize) -> Row {
        self.rows[index].clone()
    }
    pub(super) fn offset(&self, index: usize) -> f32 {
        self.offsets.get(index).copied().unwrap_or(0.0)
    }
    pub(super) fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub(super) fn visible_rows(&self, top: f32, bottom: f32) -> Range<usize> {
        let first = self
            .offsets
            .partition_point(|offset| *offset <= top)
            .saturating_sub(1);
        let end = self
            .offsets
            .partition_point(|offset| *offset < bottom)
            .min(self.rows.len());
        first.saturating_sub(1)..(end + 1).min(self.rows.len())
    }

    pub(super) fn record_height(&mut self, index: usize, height: f32) -> bool {
        let height = height.max(16.0) + ROW_GAP;
        if (self.rows[index].height - height).abs() < 0.5 {
            return false;
        }
        self.rows[index].height = height;
        true
    }

    pub(super) fn finish_measurements(&mut self, changed: Option<usize>) {
        if let Some(first) = changed {
            self.rebuild_offsets(first);
        }
    }
}

fn estimate(units: f32, width: f32) -> f32 {
    (units / width.max(60.0)).ceil().max(1.0) * 19.0 + ROW_GAP
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[test]
    fn large_history_selects_only_visible_rows_and_preserves_unicode() {
        let message = "校园网原始响应 🙂 abcdefg\n".repeat(200);
        let logs: VecDeque<_> = (0..1000)
            .map(|_| LogEntry {
                elapsed: Duration::ZERO,
                message: message.clone(),
                failed: false,
            })
            .collect();
        let mut view = LogView::default();
        view.sync(&logs, false, 520.0, 1.0);
        let range = view.visible_rows(view.total_height() - 250.0, view.total_height());
        assert!(range.len() < 12, "{} visible rows", range.len());
        let reconstructed: String = view
            .rows
            .iter()
            .filter(|r| r.entry == 0)
            .map(|r| &message[r.bytes.clone()])
            .collect();
        assert_eq!(reconstructed, message);
        assert!(
            view.rows
                .iter()
                .all(|r| message[r.bytes.clone()].chars().count() <= CHUNK_CHARS)
        );
    }
    #[test]
    fn filtering_append_clear_and_resize_rebuild_correctly() {
        let mut logs: VecDeque<_> = [false, true]
            .iter()
            .map(|failed| LogEntry {
                elapsed: Duration::ZERO,
                message: "diagnostic response".into(),
                failed: *failed,
            })
            .collect();
        let mut view = LogView::default();
        view.sync(&logs, true, 400.0, 1.0);
        assert_eq!(view.rows.len(), 1);
        assert_eq!(view.rows[0].entry, 1);
        logs.push_back(LogEntry {
            elapsed: Duration::ZERO,
            message: "错误".repeat(300),
            failed: true,
        });
        view.sync(&logs, true, 200.0, 1.5);
        assert!(view.rows.len() > 1);
        logs.clear();
        view.clear();
        view.sync(&logs, false, 400.0, 1.0);
        assert!(view.is_empty());
        assert_eq!(view.total_height(), 0.0);
    }
}
