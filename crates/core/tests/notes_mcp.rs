use duragon_core::ai::notes_mcp::serve;
use duragon_core::db::Db;
use duragon_core::doc;
use serde_json::{json, Value};

fn note(db: &Db, title: &str, body: &str) -> String {
    let n = db.create_note(Some(title)).unwrap();
    let document = doc::doc(vec![doc::title(title), doc::paragraph(body)]);
    db.save_note(&n.meta.id, title, &document, &format!("{title}\n{body}")).unwrap();
    n.meta.id
}

/// Sends each request line through `serve` and returns its replies, parsed.
fn session(db: &Db, exclude: &str, requests: &[Value]) -> Vec<Value> {
    let input: String = requests.iter().map(|r| format!("{r}\n")).collect();
    let mut out = Vec::new();
    serve(db, exclude, input.as_bytes(), &mut out).unwrap();
    String::from_utf8(out).unwrap().lines().map(|l| serde_json::from_str(l).unwrap()).collect()
}

fn call(id: i64, name: &str, arguments: Value) -> Value {
    json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}})
}

fn text(reply: &Value) -> &str {
    reply["result"]["content"][0]["text"].as_str().unwrap()
}

#[test]
fn a_chat_session_finds_and_reads_other_notes_but_never_the_open_one() {
    let db = Db::open_in_memory().unwrap();
    let open = note(&db, "Open addressing", "Linear probing in a hash map clusters keys together.");
    let other = note(&db, "Hash maps", "Robin Hood hashing evens out probe lengths in a hash map.");
    note(&db, "Groceries", "Eggs, flour, butter.");

    let replies = session(&db, &open, &[
        json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"t","version":"0"}}}),
        json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        json!({"jsonrpc":"2.0","id":2,"method":"tools/list"}),
        call(3, "search_notes", json!({"query": "hash map"})),
        call(4, "read_note", json!({"id": other})),
        call(5, "read_note", json!({"id": open})),
        call(6, "read_note", json!({"id": "nope"})),
        call(7, "search_notes", json!({"query": "zebra"})),
        json!({"jsonrpc":"2.0","id":8,"method":"ping"}),
        json!({"jsonrpc":"2.0","id":9,"method":"resources/list"}),
    ]);

    let ids: Vec<i64> = replies.iter().map(|r| r["id"].as_i64().unwrap()).collect();
    assert_eq!(ids, [1, 2, 3, 4, 5, 6, 7, 8, 9], "the notification gets no reply");

    assert_eq!(replies[0]["result"]["protocolVersion"], "2025-06-18");
    assert!(replies[0]["result"]["capabilities"]["tools"].is_object());

    let names: Vec<&str> = replies[1]["result"]["tools"].as_array().unwrap().iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(names, ["search_notes", "read_note"]);

    let found = text(&replies[2]);
    assert!(found.contains(&format!("id: {other}")), "{found}");
    assert!(found.contains("title: Hash maps"), "{found}");
    assert!(found.contains("Robin Hood"), "excerpt: {found}");
    let edited = found.lines().find_map(|l| l.strip_prefix("edited: ")).unwrap();
    assert!(edited.len() == 10 && edited.chars().enumerate().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() }), "{edited}");
    assert!(!found.contains(&open), "the open note is never a hit: {found}");
    assert!(!found.contains("Groceries"), "{found}");

    assert_eq!(replies[3]["result"]["isError"], false);
    assert!(text(&replies[3]).starts_with("Title: Hash maps"));
    assert!(text(&replies[3]).contains("Robin Hood hashing evens out probe lengths"));

    assert_eq!(replies[4]["result"]["isError"], true);
    assert!(!text(&replies[4]).contains("Linear probing"));
    assert_eq!(replies[5]["result"]["isError"], true);

    assert!(text(&replies[6]).contains("No other notes matched"));
    assert!(replies[7]["result"].is_object());
    assert_eq!(replies[8]["error"]["code"], -32601);
}

#[test]
fn a_long_note_is_cut_with_a_marker() {
    let db = Db::open_in_memory().unwrap();
    let long = note(&db, "Long", &"word ".repeat(5000));
    let replies = session(&db, "", &[call(1, "read_note", json!({"id": long}))]);
    let t = text(&replies[0]);
    assert!(t.ends_with("[truncated: the note continues]"));
    assert!(t.chars().count() < 8200, "{}", t.chars().count());
}

#[test]
fn user_text_cannot_inject_fts_syntax() {
    let db = Db::open_in_memory().unwrap();
    note(&db, "Hash maps", "Robin Hood hashing.");
    let replies = session(&db, "", &[call(1, "search_notes", json!({"query": "\"robin\" OR NEAR( ) *"}))]);
    assert_eq!(replies[0]["result"]["isError"], false, "{}", replies[0]);
}
