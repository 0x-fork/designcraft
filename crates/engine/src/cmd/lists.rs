//! Type › Bulleted & Numbered Lists › Define Lists: named lists whose numbers can continue from
//! story to story. Paragraphs join one with `listName` (and restart with `startAt`).

use designcraft_doc::{ListType, NumberedList};
use serde_json::{Value, json};

use super::{CommandSpec, bad, cmd, has_doc, has_text_or_frames, str_param};
use crate::{Result, Session};

/// The hanging indent a list gets from the Bulleted and Numbered List buttons, as InDesign's
/// default (Left Indent 0.25 in, First Line Indent −0.25 in): the bullet or number at the margin,
/// the text after its tab at the left indent.
pub const LIST_HANG: f64 = 18.0;

pub fn specs() -> Vec<CommandSpec> {
    vec![
        cmd!(
            "list.bullets",
            "Apply Bullets",
            ["Type", "Bulleted & Numbered Lists"],
            None,
            "{on?: bool (default: off when every selected paragraph has bullets, else on)} — the Bulleted List button: paragraphs with no indents get the default hanging indent (left 18 pt, first line −18 pt); taking bullets off removes that indent if it is still as applied",
            has_text_or_frames,
            |s, p| apply_list(s, p, ListType::Bullets)
        ),
        cmd!(
            "list.numbers",
            "Apply Numbers",
            ["Type", "Bulleted & Numbered Lists"],
            None,
            "{on?: bool (default: off when every selected paragraph is numbered, else on)} — the Numbered List button, as list.bullets",
            has_text_or_frames,
            |s, p| apply_list(s, p, ListType::Numbers)
        ),
        cmd!(
            "list.define",
            "Define Lists…",
            ["Type", "Bulleted & Numbered Lists"],
            None,
            "{name, continueAcrossStories?: true, rename?} — paragraphs join it with type.para {listType: numbers, listName}",
            has_doc,
            |s, p| {
                let name = str_param(p, "name").ok_or_else(|| bad("list.define", "missing name"))?.to_string();
                let cont = p.get("continueAcrossStories").and_then(Value::as_bool);
                let rename = str_param(p, "rename").map(str::to_string);
                s.edit(|d, _| {
                    let lists = &mut d.settings.lists;
                    match lists.iter_mut().find(|l| l.name == name) {
                        Some(l) => {
                            if let Some(c) = cont {
                                l.continue_across_stories = c;
                            }
                            if let Some(r) = &rename {
                                l.name = r.clone();
                            }
                        }
                        None => {
                            lists.push(NumberedList { name: rename.clone().unwrap_or(name.clone()), continue_across_stories: cont.unwrap_or(true) })
                        }
                    }
                    Ok(json!({"lists": d.settings.lists}))
                })
            }
        ),
        cmd!("list.delete", "Delete List", [], None, "{name}", has_doc, |s, p| {
            let name = str_param(p, "name").ok_or_else(|| bad("list.delete", "missing name"))?.to_string();
            s.edit(|d, _| {
                d.settings.lists.retain(|l| l.name != name);
                Ok(json!({"lists": d.settings.lists}))
            })
        }),
    ]
}

/// Bullets or numbers on (or off) for the selected paragraphs, as InDesign's Bulleted and
/// Numbered List buttons: one undo step. Values equal to the paragraph style's are left to the
/// style rather than set locally, so a paragraph back to its style shows no override.
fn apply_list(s: &mut Session, p: &Value, kind: ListType) -> Result<Value> {
    let name = if kind == ListType::Bullets { "list.bullets" } else { "list.numbers" };
    let on = match p.get("on") {
        None | Some(Value::Null) => None,
        Some(v) => Some(v.as_bool().ok_or_else(|| bad(name, "`on` must be true or false"))?),
    };
    let targets = super::text::format_targets_pub(s);
    if targets.is_empty() {
        return Err(bad(name, "select text or a text frame"));
    }
    let d = &s.doc()?.doc;
    // Off when every selected paragraph already has this list, as the button shows it pressed.
    let on = on.unwrap_or_else(|| {
        !targets.iter().all(|t| {
            d.text_story(t.story, t.cell).is_some_and(|st| {
                let (a, b) = (st.para_at(t.range.start), st.para_at(t.range.end.max(t.range.start)));
                st.paras.get(a..=b).is_some_and(|ps| ps.iter().all(|pf| d.styles.resolve_para(pf).0.list_type == kind))
            })
        })
    });
    s.edit(|d, _| {
        let styles = d.styles.clone();
        for t in &targets {
            let Some(st) = d.text_story_mut(t.story, t.cell) else { continue };
            st.format_paras(t.range.clone(), |pf| {
                let (now, _) = styles.resolve_para(pf);
                let (style, _) = styles.resolve_para_style(&pf.style);
                // A value: left to the style when it gives the same, else set here.
                let mut set = |key: &str, v: Value, same: bool| {
                    let _ = pf.para.set_json(key, if same { &Value::Null } else { &v });
                };
                let list = if on { kind } else { ListType::None };
                set("listType", serde_json::to_value(list).unwrap_or(Value::Null), style.list_type == list);
                if on && now.left_indent == 0.0 && now.first_line_indent == 0.0 {
                    set("leftIndent", json!(LIST_HANG), style.left_indent == LIST_HANG);
                    set("firstLineIndent", json!(-LIST_HANG), style.first_line_indent == -LIST_HANG);
                } else if !on && now.list_type == kind && now.left_indent == LIST_HANG && now.first_line_indent == -LIST_HANG {
                    set("leftIndent", json!(0.0), style.left_indent == 0.0);
                    set("firstLineIndent", json!(0.0), style.first_line_indent == 0.0);
                }
            });
        }
        Ok(json!({"on": on}))
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::Session;

    /// The Bulleted List button, as InDesign's: bullets with the default hanging indent (the
    /// bullet at the margin, the text 18 pt in); again, bullets and that indent go, back to the
    /// style. Numbers swap in keeping the indent; paragraphs with their own indents keep them.
    #[test]
    fn list_buttons_apply_the_default_hanging_indent() {
        let mut s = Session::new();
        s.execute("file.new", &json!({})).unwrap();
        let f = s.execute("frame.create", &json!({"rect": [72, 72, 400, 300], "content": "text", "text": "Eggs\nMilk\nFlour"})).unwrap();
        let sid = f["story"].as_u64().unwrap();
        s.execute("text.select", &json!({"story": sid, "anchor": 0, "focus": 9})).unwrap();
        let para = |s: &Session, i: usize| {
            let d = &s.doc().unwrap().doc;
            let st = d.story(designcraft_doc::StoryId(sid)).unwrap();
            (d.styles.resolve_para(&st.paras[i]).0, st.paras[i].para.clone())
        };
        let undo = s.doc().unwrap().history.undo.len();
        assert_eq!(s.execute("list.bullets", &json!({})).unwrap()["on"], true);
        for i in 0..2 {
            let (p, _) = para(&s, i);
            assert_eq!((p.list_type, p.left_indent, p.first_line_indent), (designcraft_doc::ListType::Bullets, 18.0, -18.0));
        }
        assert_eq!(para(&s, 2).0.list_type, designcraft_doc::ListType::None, "the paragraph outside the selection");
        assert_eq!(s.doc().unwrap().history.undo.len(), undo + 1, "one undo step");
        // The number of a bullet's tab: the text starts at the left indent.
        let d = &s.doc().unwrap().doc;
        let cs = designcraft_compose::compose_story(d, designcraft_doc::StoryId(sid), &Default::default());
        let line = &cs.frames[0].lines[0];
        let text_x = line.glyphs.iter().filter(|g| g.len > 0).map(|g| g.x).next().unwrap();
        assert!((text_x - line.x0 - 18.0).abs() < 0.5, "the text at the hanging indent: {} from {}", text_x, line.x0);
        // Numbers instead: the indent stays.
        s.execute("list.numbers", &json!({})).unwrap();
        let (p, _) = para(&s, 0);
        assert_eq!((p.list_type, p.left_indent), (designcraft_doc::ListType::Numbers, 18.0));
        // Numbers off: list and hanging indent go, and nothing is left set on the paragraph.
        assert_eq!(s.execute("list.numbers", &json!({})).unwrap()["on"], false);
        let (p, own) = para(&s, 0);
        assert_eq!((p.list_type, p.left_indent, p.first_line_indent), (designcraft_doc::ListType::None, 0.0, 0.0));
        assert_eq!(own, designcraft_doc::ParaAttrs::default(), "back to the style");
        // A paragraph with its own indents keeps them.
        s.execute("type.para", &json!({"attrs": {"leftIndent": 36}})).unwrap();
        s.execute("list.bullets", &json!({"on": true})).unwrap();
        let (p, _) = para(&s, 0);
        assert_eq!((p.left_indent, p.first_line_indent), (36.0, 0.0));
        s.execute("list.bullets", &json!({"on": false})).unwrap();
        assert_eq!(para(&s, 0).0.left_indent, 36.0);
        assert!(s.execute("list.bullets", &json!({"on": "yes"})).is_err());
    }

    /// The digit the story's first line starts with (its list number).
    fn digit(g: &designcraft_compose::PlacedGlyph) -> char {
        let face = g.face.get();
        ('0'..='9').find(|c| face.glyph_for(*c) == g.gid).unwrap_or('?')
    }

    fn first_char(s: &Session, sid: u64) -> char {
        let d = &s.doc().unwrap().doc;
        let cs = designcraft_compose::compose_story(d, designcraft_doc::StoryId(sid), &Default::default());
        digit(&cs.frames[0].lines[0].glyphs[0])
    }

    #[test]
    fn named_lists_continue_across_stories() {
        let mut s = Session::new();
        s.execute("file.new", &json!({"pages": 2, "facingPages": false})).unwrap();
        s.execute("list.define", &json!({"name": "Steps"})).unwrap();
        let mut stories = vec![];
        for (spread, text) in [(0, "Mix\nStir"), (1, "Bake\nServe")] {
            let r = s.execute("frame.create", &json!({"spread": spread, "rect": [72, 72, 400, 300], "content": "text", "text": text})).unwrap();
            let sid = r["story"].as_u64().unwrap();
            s.execute("text.select", &json!({"story": sid, "anchor": 0, "focus": text.len()})).unwrap();
            s.execute("type.para", &json!({"attrs": {"listType": "numbers", "listName": "Steps"}})).unwrap();
            stories.push(sid);
        }
        assert_eq!(first_char(&s, stories[0]), '1');
        assert_eq!(first_char(&s, stories[1]), '3', "carries on from the first story");
        // Not continuing: each story starts at 1.
        s.execute("list.define", &json!({"name": "Steps", "continueAcrossStories": false})).unwrap();
        assert_eq!(first_char(&s, stories[1]), '1');
        s.execute("list.define", &json!({"name": "Steps", "continueAcrossStories": true})).unwrap();
        // The session's cache follows earlier stories (no stale numbers).
        s.execute("text.select", &json!({"story": stories[0], "anchor": 0, "focus": 0})).unwrap();
        s.execute("text.insert", &json!({"text": "Prep\n"})).unwrap();
        let d = s.doc().unwrap().doc.clone();
        let cs = s.cache.get(&d, designcraft_doc::StoryId(stories[1]), None);
        assert_eq!(digit(&cs.frames[0].lines[0].glyphs[0]), '4');
        // Start At restarts.
        s.execute("text.select", &json!({"story": stories[1], "anchor": 0, "focus": 0})).unwrap();
        s.execute("type.para", &json!({"attrs": {"startAt": 7}})).unwrap();
        assert_eq!(first_char(&s, stories[1]), '7');
        // null removes the override again.
        s.execute("type.para", &json!({"attrs": {"startAt": null}})).unwrap();
        assert_eq!(first_char(&s, stories[1]), '4');
    }
}
