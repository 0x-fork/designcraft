//! Type › Notes: editorial notes anchored in text (never printed or exported).

use std::sync::Arc;

use designcraft_doc::{ANCHOR_MARK, ENDNOTE_REF, EditorialNote, FOOTNOTE_REF, INDEX_MARK, NOTE_MARK, OBJECT_MARK, StoryId, TABLE_ANCHOR, XREF_MARK};
use serde_json::{Value, json};

use super::{CommandSpec, bad, cmd, has_doc, has_text, ok, str_param};
use crate::{Result, Session};

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!("note.new", "New Note", ["Type", "Notes"], None, "{text, author?} — at the insertion point → {story, id}", has_text, |s, p| {
            let st = s.doc()?;
            let ts = st.selection.text.ok_or_else(|| bad("note.new", "place the insertion point in text"))?;
            if ts.cell.is_some() {
                return Err(bad("note.new", "notes go in story text"));
            }
            let text = super::text_param(p, "text");
            let author = str_param(p, "author").unwrap_or("").to_string();
            let at = ts.anchor.max(ts.focus);
            s.edit(|d, sel| {
                let story = d.story_mut(ts.story).ok_or_else(|| bad("note.new", "no such story"))?;
                let fmt = story.char_format_at(at).clone();
                story.insert_with(at, &NOTE_MARK.to_string(), fmt);
                // A file may hold fewer notes than anchors; give every anchor one.
                story.fix_marks();
                let k = story.text[..at].matches(NOTE_MARK).count();
                let n = story.editorial.get_mut(k).ok_or_else(|| bad("note.new", "the story's notes don't match their anchors"))?;
                let n = Arc::make_mut(n);
                (n.text, n.author) = (text, author);
                let id = n.id;
                if let Some(t) = &mut sel.text {
                    let c = at + NOTE_MARK.len_utf8();
                    (t.anchor, t.focus) = (c, c);
                }
                Ok(json!({"story": ts.story.0, "id": id}))
            })
        }),
        cmd!(
            "note.convertFromText",
            "Convert to Note",
            ["Type", "Notes"],
            None,
            "{author?} → {story, id}. The selected text moves into a new note anchored where it was.",
            has_text_range,
            convert_from_text
        ),
        cmd!("note.edit", "Edit Note", [], None, "{story, id, text?, author?}", has_doc, |s, p| {
            let (sid, id) = target(p)?;
            let (text, author) = (str_param(p, "text").map(|_| super::text_param(p, "text")), str_param(p, "author").map(str::to_string));
            s.edit(|d, _| {
                let st = d.story_mut(sid).ok_or_else(|| bad("note.edit", "no such story"))?;
                let n = st.editorial.iter_mut().find(|n| n.id == id).ok_or_else(|| bad("note.edit", format!("no note {id}")))?;
                let n = Arc::make_mut(n);
                if let Some(t) = text {
                    n.text = t;
                }
                if let Some(a) = author {
                    n.author = a;
                }
                st.rev = st.rev.wrapping_add(1);
                ok()
            })
        }),
        cmd!("note.delete", "Delete Note", ["Type", "Notes"], None, "{story, id}", has_doc, |s, p| {
            let (sid, id) = target(p)?;
            s.edit(|d, _| {
                let st = d.story_mut(sid).ok_or_else(|| bad("note.delete", "no such story"))?;
                let at = anchor(st, id).ok_or_else(|| bad("note.delete", format!("no note {id}")))?;
                st.delete(at..at + NOTE_MARK.len_utf8());
                ok()
            })
        }),
        cmd!(
            "note.convertToText",
            "Convert Note to Text",
            ["Type", "Notes"],
            None,
            "{story, id} — the note's text replaces its anchor",
            has_doc,
            |s, p| {
                let (sid, id) = target(p)?;
                s.edit(|d, _| {
                    let st = d.story_mut(sid).ok_or_else(|| bad("note.convertToText", "no such story"))?;
                    let at = anchor(st, id).ok_or_else(|| bad("note.convertToText", format!("no note {id}")))?;
                    let text = st.editorial.iter().find(|n| n.id == id).map(|n| n.text.clone()).unwrap_or_default();
                    st.replace(at..at + NOTE_MARK.len_utf8(), &text);
                    ok()
                })
            }
        ),
        cmd!(query "note.list", "Notes", [], None, "{} → [{story, id, at, author, text}] in story order", has_doc, |s, _| {
            let d = &s.doc()?.doc;
            let mut out = Vec::new();
            for st in d.stories.values() {
                for (k, (at, _)) in st.text.match_indices(NOTE_MARK).enumerate() {
                    if let Some(n) = st.editorial.get(k) {
                        out.push(json!({"story": st.id.0, "id": n.id, "at": at, "author": n.author, "text": n.text}));
                    }
                }
            }
            Ok(Value::Array(out))
        }),
    ]
}

/// Characters whose payload lives outside the story text: a note made from text that holds one
/// would lose it.
const MARKS: [char; 8] = [NOTE_MARK, ENDNOTE_REF, FOOTNOTE_REF, OBJECT_MARK, INDEX_MARK, ANCHOR_MARK, XREF_MARK, TABLE_ANCHOR];

fn has_text_range(s: &Session) -> std::result::Result<(), String> {
    has_text(s)?;
    match s.active().and_then(|d| d.selection.text) {
        Some(t) if t.cell.is_some() => Err("notes go in story text".into()),
        Some(t) if t.is_caret() => Err("select the text to convert".into()),
        _ => Ok(()),
    }
}

/// Type › Notes › Convert to Note: the selected text leaves the story and becomes the text of a
/// new note, whose anchor takes the text's place.
fn convert_from_text(s: &mut Session, p: &Value) -> Result<Value> {
    const ID: &str = "note.convertFromText";
    let ts = s.doc()?.selection.text.ok_or_else(|| bad(ID, "select the text to convert"))?;
    if ts.cell.is_some() {
        return Err(bad(ID, "notes go in story text"));
    }
    let r = ts.range();
    if r.is_empty() {
        return Err(bad(ID, "select the text to convert"));
    }
    let author = str_param(p, "author").unwrap_or("").to_string();
    s.edit(|d, sel| {
        let story = d.story_mut(ts.story).ok_or_else(|| bad(ID, "no such story"))?;
        let text = story.text.get(r.clone()).ok_or_else(|| bad(ID, "the selection is outside the story"))?.to_string();
        if text.contains(MARKS) {
            return Err(bad(ID, "the selection holds a table, an anchored object, a note, a footnote, an endnote or a marker"));
        }
        let fmt = story.format_after(r.start).clone();
        story.delete(r.clone());
        story.insert_with(r.start, &NOTE_MARK.to_string(), fmt);
        story.fix_marks();
        let k = story.text[..r.start].matches(NOTE_MARK).count();
        let n = story.editorial.get_mut(k).ok_or_else(|| bad(ID, "the story's notes don't match their anchors"))?;
        let n = Arc::make_mut(n);
        (n.text, n.author) = (text, author);
        let id = n.id;
        if let Some(t) = &mut sel.text {
            let c = r.start + NOTE_MARK.len_utf8();
            (t.anchor, t.focus) = (c, c);
        }
        Ok(json!({"story": ts.story.0, "id": id}))
    })
}

fn target(p: &Value) -> Result<(StoryId, u64)> {
    let sid = p.get("story").and_then(Value::as_u64).ok_or_else(|| bad("note", "`story` required"))?;
    let id = p.get("id").and_then(Value::as_u64).ok_or_else(|| bad("note", "`id` required"))?;
    Ok((StoryId(sid), id))
}

/// Byte offset of note `id`'s anchor.
fn anchor(st: &designcraft_doc::Story, id: u64) -> Option<usize> {
    let k = st.editorial.iter().position(|n: &Arc<EditorialNote>| n.id == id)?;
    st.text.match_indices(NOTE_MARK).nth(k).map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::Session;

    #[test]
    fn notes_are_anchored_unprinted_and_convertible() {
        let mut s = Session::new();
        s.execute("file.new", &json!({})).unwrap();
        let r = s.execute("frame.create", &json!({"rect": [72, 72, 400, 300], "content": "text", "text": "Draft copy."})).unwrap();
        let sid = designcraft_doc::StoryId(r["story"].as_u64().unwrap());
        let end_x = |s: &Session| s.cache.get(&s.doc().unwrap().doc, sid, None).frames[0].lines[0].end_x;
        let before = end_x(&s);
        s.execute("text.select", &json!({"story": r["story"], "anchor": 5, "focus": 5})).unwrap();
        let n = s.execute("note.new", &json!({"text": "check this", "author": "Ed"})).unwrap();
        assert!((end_x(&s) - before).abs() < 1e-6, "a note takes no space");
        let list = s.execute("note.list", &json!({})).unwrap();
        assert_eq!(list[0]["text"], "check this");
        assert_eq!(list[0]["at"], 5);
        let txt = s.execute("file.exportText", &json!({"story": r["story"]})).unwrap();
        assert_eq!(txt["text"], "Draft copy.", "not exported");
        s.execute("note.edit", &json!({"story": n["story"], "id": n["id"], "text": " (checked)"})).unwrap();
        s.execute("note.convertToText", &json!({"story": n["story"], "id": n["id"]})).unwrap();
        assert_eq!(s.doc().unwrap().doc.stories[&sid].text, "Draft (checked) copy.");
        assert!(s.doc().unwrap().doc.stories[&sid].editorial.is_empty());
    }

    #[test]
    fn notes_insert_into_a_story_whose_markers_lack_payloads() {
        let mut s = Session::new();
        s.execute("file.new", &json!({})).unwrap();
        let r = s.execute("frame.create", &json!({"rect": [72, 72, 400, 300], "content": "text", "text": "Draft copy."})).unwrap();
        let sid = designcraft_doc::StoryId(r["story"].as_u64().unwrap());
        // A damaged file: a note anchor, an endnote and a footnote reference without payloads.
        let st = std::sync::Arc::make_mut(&mut s.doc_mut().unwrap().doc).story_mut(sid).unwrap();
        st.insert(0, &format!("{}{}{}", designcraft_doc::NOTE_MARK, designcraft_doc::ENDNOTE_REF, designcraft_doc::FOOTNOTE_REF));
        st.editorial.clear();
        st.endnotes.clear();
        st.notes.clear();
        let end = st.len();
        s.execute("text.select", &json!({"story": r["story"], "anchor": end, "focus": end})).unwrap();
        s.execute("note.new", &json!({"text": "check this"})).unwrap();
        s.execute("endnote.insert", &json!({"text": "An endnote."})).unwrap();
        s.execute("footnote.insert", &json!({"text": "A footnote."})).unwrap();
        let st = &s.doc().unwrap().doc.stories[&sid];
        st.check().unwrap();
        assert_eq!(st.editorial[1].text, "check this");
        assert_eq!(st.endnotes[1].text.text, "An endnote.");
        assert_eq!(st.notes[1].text.text, "A footnote.");
    }

    #[test]
    fn selected_text_converts_to_a_note_at_its_place() {
        let mut s = Session::new();
        s.execute("file.new", &json!({})).unwrap();
        let r = s.execute("frame.create", &json!({"rect": [72, 72, 400, 300], "content": "text", "text": "One two\nthree four."})).unwrap();
        let sid = designcraft_doc::StoryId(r["story"].as_u64().unwrap());
        let mark = designcraft_doc::NOTE_MARK.len_utf8();
        s.execute("text.select", &json!({"story": r["story"], "anchor": 3, "focus": 3})).unwrap();
        s.execute("note.new", &json!({"text": "earlier"})).unwrap();
        // "two\nthree " across the paragraph return, selected backwards.
        let (a, b) = (3 + mark + 1, 3 + mark + 1 + "two\nthree ".len());
        s.execute("text.select", &json!({"story": r["story"], "anchor": b, "focus": a})).unwrap();
        let n = s.execute("note.convertFromText", &json!({"author": "Ed"})).unwrap();
        let st = &s.doc().unwrap().doc.stories[&sid];
        st.check().unwrap();
        assert_eq!(st.text, format!("One{m} {m}four.", m = designcraft_doc::NOTE_MARK));
        assert_eq!(st.paras.len(), 1, "the paragraph return went into the note");
        let list = s.execute("note.list", &json!({})).unwrap();
        assert_eq!(list[0]["text"], "earlier");
        assert_eq!((list[1]["id"].clone(), list[1]["at"].clone()), (n["id"].clone(), json!(a)));
        assert_eq!((list[1]["text"].as_str(), list[1]["author"].as_str()), (Some("two\nthree "), Some("Ed")));
        let caret = s.doc().unwrap().selection.text.map(|t| (t.anchor, t.focus));
        assert_eq!(caret, Some((a + mark, a + mark)), "the insertion point follows the anchor");
        s.execute("note.convertToText", &json!({"story": n["story"], "id": n["id"]})).unwrap();
        assert_eq!(s.doc().unwrap().doc.stories[&sid].text, format!("One{} two\nthree four.", designcraft_doc::NOTE_MARK));
        s.execute("edit.undo", &json!({})).unwrap();
        s.execute("edit.undo", &json!({})).unwrap();
        let st = &s.doc().unwrap().doc.stories[&sid];
        assert_eq!((st.text.as_str(), st.editorial.len()), (format!("One{} two\nthree four.", designcraft_doc::NOTE_MARK).as_str(), 1));
    }

    #[test]
    fn convert_to_note_needs_selected_story_text_without_objects() {
        let mut s = Session::new();
        s.execute("file.new", &json!({})).unwrap();
        let r = s.execute("frame.create", &json!({"rect": [72, 72, 400, 300], "content": "text", "text": "Draft copy."})).unwrap();
        let sid = designcraft_doc::StoryId(r["story"].as_u64().unwrap());
        s.execute("text.select", &json!({"story": r["story"], "anchor": 5, "focus": 5})).unwrap();
        let disabled = |r| matches!(r, Err(crate::EngineError::Disabled(..)));
        assert!(disabled(s.execute("note.convertFromText", &json!({}))), "an insertion point selects no text");
        s.execute("note.new", &json!({"text": "keep"})).unwrap();
        let before = s.doc().unwrap().doc.stories[&sid].clone();
        let all = before.len();
        s.execute("text.select", &json!({"story": r["story"], "anchor": 0, "focus": all})).unwrap();
        let refused = s.execute("note.convertFromText", &json!({}));
        assert!(matches!(refused, Err(crate::EngineError::BadParams { .. })), "a note inside the selection would be lost");
        assert_eq!(s.doc().unwrap().doc.stories[&sid], before);
        s.execute("text.select", &json!({"story": r["story"], "anchor": all, "focus": all})).unwrap();
        s.execute("table.insert", &json!({"rows": 1, "cols": 1})).unwrap();
        s.execute("text.insert", &json!({"text": "cell", "raw": true})).unwrap();
        s.execute("text.move", &json!({"dir": "left", "extend": true})).unwrap();
        assert!(s.doc().unwrap().selection.text.is_some_and(|t| t.cell.is_some() && !t.is_caret()));
        assert!(disabled(s.execute("note.convertFromText", &json!({}))), "notes go in story text");
    }
}
