//! Domain types shared with the frontend (mirrors `src/lib/api/types.ts`).
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

pub const BUILTIN_RIDDLER_ID: &str = "00000000-0000-4000-8000-000000000001";

macro_rules! str_enum {
    ($(#[$m:meta])* $name:ident { $($v:ident => $s:literal),+ $(,)? }) => {
        $(#[$m])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub enum $name { $(#[serde(rename = $s)] $v),+ }
        impl $name {
            pub fn as_str(self) -> &'static str {
                match self { $(Self::$v => $s),+ }
            }
            pub fn parse(s: &str) -> Option<Self> {
                match s { $($s => Some(Self::$v),)+ _ => None }
            }
        }
    };
}

str_enum!(ReadingMode { Thinks => "thinks", Asks => "asks" });
str_enum!(Tone { Plain => "plain", Warm => "warm", Blunt => "blunt", Riddle => "riddle" });
str_enum!(Frequency { Rarely => "rarely", Matters => "matters", Often => "often" });
str_enum!(ThreadKind { Question => "question", Reply => "reply" });
str_enum!(ThreadStatus { Open => "open", Resolved => "resolved", Dismissed => "dismissed" });
str_enum!(DrawingState { None => "none", Pending => "pending", Done => "done", Failed => "failed" });
str_enum!(Author { You => "you", Riddled => "riddled" });
str_enum!(ReadingPace { Regular => "regular", Slow => "slow" });
str_enum!(QuestionPlacement { Both => "both", Right => "right", Marks => "marks" });
str_enum!(Appearance { System => "system", Light => "light", Dark => "dark" });
str_enum!(TextSize { Small => "small", Default => "default", Large => "large" });
str_enum!(Lettering { Own => "own", Notes => "notes" });
str_enum!(InkColour {
    Ink => "ink", Blue => "blue", Green => "green", Red => "red",
    Purple => "purple", Brown => "brown", Orange => "orange",
});
str_enum!(ExportFormat { Md => "md", Docx => "docx" });
str_enum!(AiTask {
    Read => "read", Answer => "answer", Reply => "reply",
    Chat => "chat", Ask => "ask", Visualize => "visualize", Draw => "draw", Learn => "learn",
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteMeta {
    pub id: String,
    pub title: String,
    pub folder_id: Option<String>,
    pub updated_at: i64,
    pub has_open_question: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    #[serde(flatten)]
    pub meta: NoteMeta,
    pub doc: Value,
    pub riddler_id: String,
    pub mode: ReadingMode,
    pub voice_on: bool,
    pub web_on: bool,
    pub brief: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,
    pub note_id: String,
    pub thread_id: Option<String>,
    pub author: Author,
    pub text: String,
    pub drawing: DrawingState,
    pub svg: Option<String>,
    /// Pages the model used when web was on; empty otherwise.
    #[serde(default)]
    pub sources: Vec<Source>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Source {
    pub url: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    pub id: String,
    pub note_id: String,
    pub kind: ThreadKind,
    pub quote: String,
    pub status: ThreadStatus,
    pub pinned: bool,
    pub created_at: i64,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Riddler {
    pub id: String,
    pub name: String,
    pub description: String,
    pub how_it_thinks: String,
    pub tone: Tone,
    pub frequency: Frequency,
    pub builtin: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub new_note_mode: ReadingMode,
    pub reading_pace: ReadingPace,
    pub question_placement: QuestionPlacement,
    pub web_in_new_notes: bool,
    pub voice_in_new_notes: bool,
    pub voice_name: String,
    pub speaking_speed: f64,
    pub appearance: Appearance,
    pub text_size: TextSize,
    pub page_width: Option<f64>,
    pub lettering: Lettering,
    pub ink_colour: InkColour,
    pub ink_from_pointer: bool,
    pub just_my_words: bool,
    pub about_you: String,
    pub learn_about_you: bool,
    pub read_model: String,
    pub chat_model: String,
    pub draw_model: String,
    pub sidebar_open: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            new_note_mode: ReadingMode::Thinks,
            reading_pace: ReadingPace::Regular,
            question_placement: QuestionPlacement::Both,
            web_in_new_notes: false,
            voice_in_new_notes: false,
            voice_name: "bm_george".into(),
            speaking_speed: 1.0,
            appearance: Appearance::System,
            text_size: TextSize::Default,
            page_width: None,
            lettering: Lettering::Own,
            ink_colour: InkColour::Ink,
            ink_from_pointer: false,
            just_my_words: false,
            about_you: String::new(),
            learn_about_you: true,
            read_model: "sonnet".into(),
            chat_model: "sonnet".into(),
            draw_model: "opus".into(),
            sidebar_open: false,
        }
    }
}

/// Distinguishes an absent field (`None`) from an explicit `null` (`Some(None)`).
fn present<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NoteMetaPatch {
    pub riddler_id: Option<String>,
    pub mode: Option<ReadingMode>,
    pub voice_on: Option<bool>,
    pub web_on: Option<bool>,
    pub brief: Option<String>,
    #[serde(default, deserialize_with = "present")]
    pub folder_id: Option<Option<String>>,
}
