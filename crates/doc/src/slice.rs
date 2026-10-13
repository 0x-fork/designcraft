//! Formatted text slices: copy a range of a story (with paragraph and character formats,
//! footnotes, cross-references, index markers and tables) and insert it into another story.

use std::sync::Arc;

use crate::ids::StoryId;
use crate::index::INDEX_MARK;
use crate::notes::FOOTNOTE_REF;
use crate::story::{CharRun, Story, TABLE_ANCHOR};
use crate::xref::{ANCHOR_MARK, XREF_MARK};

impl Story {
    /// A frameless story holding `range` with its formatting and the objects anchored in it.
    pub fn extract(&self, range: std::ops::Range<usize>) -> Story {
        let a = crate::story::floor_char_boundary(&self.text, range.start.min(self.len()));
        let b = crate::story::floor_char_boundary(&self.text, range.end.min(self.len())).max(a);
        let mut out = Story::new(StoryId(0));
        out.direction = self.direction;
        out.text = self.text[a..b].to_string();
        let (p0, p1) = (self.para_at(a), self.para_at(b));
        out.paras = self.paras[p0..=p1].to_vec();
        out.chars = Vec::new();
        for (r, f) in self.runs() {
            let s = r.start.max(a);
            let e = r.end.min(b);
            if s < e {
                out.chars.push(CharRun { len: e - s, format: f.clone() });
            }
        }
        if out.chars.is_empty() {
            out.chars.push(CharRun { len: 0, format: self.char_format_at(a).clone() });
        }
        let count = |m: char, upto: usize| self.text[..upto].matches(m).count();
        let span = |m: char| count(m, a)..count(m, b);
        out.notes = self.notes[span(FOOTNOTE_REF)].to_vec();
        out.anchors = self.anchors[span(ANCHOR_MARK)].to_vec();
        out.xrefs = self.xrefs[span(XREF_MARK)].to_vec();
        out.index_refs = self.index_refs[span(INDEX_MARK)].to_vec();
        out.objects = self.objects[span(crate::anchored::OBJECT_MARK)].to_vec();
        for p in &out.paras {
            if let Some(id) = p.table
                && let Some(t) = self.tables.get(&id)
                && out.text.contains(TABLE_ANCHOR)
            {
                out.tables.insert(id, t.clone());
            }
        }
        out.fix_tables();
        out
    }

    /// Insert a slice made by [`Story::extract`] at `pos`, keeping its formatting: the first
    /// pasted paragraph merges into the paragraph at `pos` (it keeps that paragraph's format, as
    /// in InDesign), later paragraphs keep their own formats. Returns the end of the insertion.
    pub fn insert_story(&mut self, pos: usize, src: &Story) -> usize {
        let pos = crate::story::floor_char_boundary(&self.text, pos.min(self.len()));
        if src.text.is_empty() {
            return pos;
        }
        // The table anchored in the paragraph at `pos` keeps its anchor wherever the paste moves it.
        let p0 = self.para_at(pos);
        let own = self.paras.get(p0).and_then(|p| p.table).and_then(|id| {
            let r = self.para_ranges().get(p0)?.clone();
            Some((r.start + self.text.get(r)?.find(TABLE_ANCHOR)?, id))
        });
        // Text first (creates empty marker entries), then formats and marker payloads; tables are
        // fixed up once at the end.
        let mut at = pos;
        for (r, f) in src.runs() {
            if r.is_empty() {
                continue;
            }
            self.insert_with_raw(at, &src.text[r.clone()], f.clone());
            at += r.len();
        }
        let end = at;
        let mut claims: Vec<(usize, u64)> = own.map(|(a, id)| (if a >= pos { a + (end - pos) } else { a }, id)).into_iter().collect();
        // Paragraph formats of the pasted paragraphs after the first.
        for (k, pf) in src.paras.iter().enumerate().skip(1) {
            if let Some(p) = self.paras.get_mut(p0 + k) {
                *p = crate::story::ParaFormat { table: None, ..pf.clone() };
            }
        }
        // Payloads of footnotes, cross-references and index markers, in order.
        let first = |m: char| self.text[..pos].matches(m).count();
        let (nf, nx, ni, na) = (first(FOOTNOTE_REF), first(XREF_MARK), first(INDEX_MARK), first(ANCHOR_MARK));
        let no = first(crate::anchored::OBJECT_MARK);
        for (k, o) in src.objects.iter().enumerate() {
            if let Some(slot) = self.objects.get_mut(no + k) {
                *slot = o.clone();
            }
        }
        for (k, n) in src.notes.iter().enumerate() {
            if let Some(slot) = self.notes.get_mut(nf + k) {
                let id = slot.id;
                *slot = Arc::new(crate::notes::Footnote { id, text: n.text.clone() });
            }
        }
        for (k, x) in src.xrefs.iter().enumerate() {
            if let Some(slot) = self.xrefs.get_mut(nx + k) {
                *slot = x.clone();
            }
        }
        for (k, r) in src.index_refs.iter().enumerate() {
            if let Some(slot) = self.index_refs.get_mut(ni + k) {
                *slot = r.clone();
            }
        }
        // Anchors are new destinations (fresh ids) with the copied names.
        for (k, a) in src.anchors.iter().enumerate() {
            if let Some(slot) = self.anchors.get_mut(na + k) {
                let id = slot.id;
                *slot = Arc::new(crate::xref::TextAnchor { id, name: a.name.clone() });
            }
        }
        // Tables: re-attach at their anchors (fresh ids so a pasted copy never shares an id).
        if !src.tables.is_empty() {
            let mut next = self.tables.keys().max().copied().unwrap_or(0) + 1;
            let ranges = src.para_ranges();
            for (pf, r) in src.paras.iter().zip(ranges) {
                let Some(tid) = pf.table else { continue };
                let Some(t) = src.tables.get(&tid) else { continue };
                let Some(k) = src.text.get(r.clone()).and_then(|t| t.find(TABLE_ANCHOR)) else { continue };
                let mut t = (**t).clone();
                t.id = next;
                next += 1;
                claims.push((pos + r.start + k, t.id));
                self.tables.insert(t.id, Arc::new(t));
            }
        }
        let splits = self.fix_tables_with(&claims);
        self.rev += 1;
        // Past any return the fix-up put inside the pasted text.
        end + splits.iter().filter(|&&s| s < end).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::story::ParaFormat;

    #[test]
    fn pasting_a_table_next_to_another_gives_each_its_own_paragraph() {
        let mut s = Story::with_text(StoryId(1), "x", ParaFormat::default());
        let a = s.insert_table(1, crate::Table::new(5, 1, 1, 0, 0, 10.0));
        let n = TABLE_ANCHOR.len_utf8();
        let copy = s.extract(a..a + n);
        // After the anchor: the copy follows the original.
        let mut after = s.clone();
        let end = after.insert_story(a + n, &copy);
        after.check().unwrap();
        assert_eq!(after.text, format!("x\n{TABLE_ANCHOR}\n{TABLE_ANCHOR}\n"));
        assert_eq!((after.table_anchor(5), after.table_anchor(6)), (Some(a), Some(a + n + 1)));
        assert_eq!(end, a + 2 * n + 1);
        // Before the anchor: the copy comes first and the original keeps its table.
        let mut before = s.clone();
        let end = before.insert_story(a, &copy);
        before.check().unwrap();
        assert_eq!((before.table_anchor(6), before.table_anchor(5)), (Some(a), Some(a + n + 1)));
        assert_eq!(end, a + n);
        // Paragraphs pasted before the anchor keep the table too.
        let mut text = s.clone();
        text.insert_story(a, &Story::with_text(StoryId(2), "Hello\nWorld", ParaFormat::default()));
        text.check().unwrap();
        assert_eq!(text.table_anchor(5), Some(a + "Hello\nWorld".len()));
    }

    #[test]
    fn copy_paste_keeps_formatting_and_notes() {
        let mut a = Story::with_text(StoryId(1), "Alpha beta\nGamma delta", ParaFormat::default());
        a.format_chars(0..5, |f| f.over.size = Some(20.0));
        a.format_paras(11..11, |p| p.style = "Head".into());
        a.insert_note(8, "A note.", ParaFormat::default());
        a.check().unwrap();
        let slice = a.extract(0..a.len());
        slice.check().unwrap();
        assert_eq!(slice.notes.len(), 1);
        let mut b = Story::with_text(StoryId(2), "Start: end", ParaFormat::default());
        let end = b.insert_story(7, &slice);
        b.check().unwrap();
        assert_eq!(&b.text[7..end], slice.text.as_str());
        assert_eq!(b.char_format_at(8).over.size, Some(20.0));
        assert_eq!(b.paras[1].style, "Head");
        assert_eq!(b.notes.len(), 1);
        assert_eq!(b.notes[0].text.text, "A note.");
        // A partial slice.
        let e = 8 + FOOTNOTE_REF.len_utf8();
        let part = a.extract(2..e);
        part.check().unwrap();
        assert_eq!(part.text, a.text[2..e]);
        assert_eq!(part.notes.len(), 1);
        assert!(a.extract(2..8).notes.is_empty());
    }
}
