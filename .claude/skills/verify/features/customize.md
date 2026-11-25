# Customize

The Customize dialog holds the reading, voice, look and model settings, plus the Minds, About you, Voice models and Help pages. Settings save as they change. There is no Save button, except for an API key and for a mind. From Minds, the writer opens the mind editor to make or change a mind.

## Sub-features

- `customize-open` opens the dialog from the keyboard and the sidebar's Settings menu.
- `customize-search` filters settings by text.
- `customize-switch` toggles a setting and stores it. On `Customize Duragon` the switches are `Web search in new notes` (`webInNewNotes`), `Search my other notes in chat` (`searchMyNotes`), `Explain symbols in formulas` (`explainSymbols`) and `New notes speak aloud` (`voiceInNewNotes`), all off by default. Under `Notes`, `Deleted notes` picks one of three radios (`deletedNotes`, `trash.md`). `About you` has `Duragon learns about you` (`learnAboutYou`, on by default).
- `customize-choices` picks between the radios of `Customize Duragon`, and stores the pick at once. The defaults are marked with a star. `newNoteMode`: `Thinks with you`* (`thinks`) or `When you ask` (`asks`). `readingPace`: `Regular`* or `Slow`. `questionPlacement`: `Both margins`*, `Right margin` or `Marks only` (`marks`). `appearance`: `System`*, `Light` or `Dark`. `textSize`: `Small`, `Default`* or `Large`. `lettering`: `Duragon’s own`* or `Like your notes` (`notes`). `inkColour`: `Ink`*, `Blue`, `Green`, `Red`, `Purple`, `Brown` or `Orange`. `voiceName`: `George`*, `Emma`, `Michael` or `Heart`, each with a button `Play a sample of …` that also picks it. `speakingSpeed` is a slider in steps of 0.05, at `1.00×`*. `Reset` by `Page width` is disabled until the width was dragged.
- `customize-about` keeps the paragraph `About you` (`aboutYou`, at most 700 characters).
- `customize-models` picks the model, its provider and the thinking level for `Reading`, `Chat` and `Drawing` (`readModel`, `readProvider`, `readThinking` and so on), at the end of `Customize Duragon`.
- `customize-providers` lists the providers that are set up or in use, and adds, changes and removes them.
- `customize-pages` moves between `Customize Duragon`, `Minds`, `About you`, `Voice models` and `Help`. Help lists the shortcuts, including ```, `Esc`, `Tab` and `Shift Tab`.
- `customize-minds` opens the mind editor from `New Mind` on the Minds page or from the mind picker in the note header.

## How to get to it (user POV)

- Press `Control+,` (`Cmd+,` on macOS). It toggles, and it reopens on the last page shown. While another dialog, such as the mind editor or the command palette, is open, it does nothing.
- Choose `Open notes`, then `Settings` at the bottom of the sidebar, then the menuitem `Customize Ctrl ,` (`find role menuitem click --name "Customize"` matches it) or `Minds`. The menu always opens on that page.
- The mind picker in the note header (button `Duragon`) has the menuitems `New Mind` and `Manage`.
- `Close`, Escape or a click on the scrim closes it, and closing clears the search. Escape closes the newest layer first, so with the mind editor open, the first Escape closes the editor and the second closes Customize.

## Driving it with agent-browser

Preconditions:

- Baseline state from `README.md`.

- **Open.** Run `agent-browser press Control+,`, `agent-browser wait '[aria-label="Settings sections"]'` and `agent-browser wait 400`. The navigation `Settings sections`, the button `Close` and the searchbox `Search settings` appear.
- **Toggle.** Run `agent-browser find role switch click --name "Explain symbols in formulas"`. The switch reports `checked=true` in `agent-browser snapshot -i`.
- **Stored.** Read `(await b.getSettings()).explainSymbols` with the store eval from `../SKILL.md`. It goes from `false` to `true`.
- **Appearance.** Choose `Dark`, then `Light`. The whole window fades in 250 ms, with `ui-fade` on `::view-transition-new(root)`. `document.getAnimations()` also lists the browser’s own animation of the group. Choosing what already shows starts no transition.
- **Choices.** Pick `When you ask`, `Slow`, `Marks only`, `Large`, `Like your notes` and `Blue`, then `Emma` by its sample button. `getSettings()` has `asks`, `slow`, `marks`, `large`, `notes`, `blue` and Emma’s voice, and `document.documentElement.dataset` has `textSize`, `lettering` and `ink`. A note made with `Control+n` then has the mode `asks`.
- **Speed and width.** Focus the slider with `agent-browser eval 'document.querySelector("[aria-modal] input[type=range]").focus()'` and press ArrowRight. `speakingSpeed` is 1.05. With the page width dragged to 770, choose `Reset`. `pageWidth` is `null`.
- **About you.** On `About you`, fill the textbox and wait 600 ms. `aboutYou` holds it, and the page counts `16 of 700`.
- **Search.** Run `agent-browser fill '[aria-label="Search settings"]' "symbols"`. Only that switch row stays, under its `Reading` heading. Rows that don't match are removed from the DOM, not hidden. "zzz" shows `Nothing matches “zzz”.`
- **Pages.** Choose each button in `Settings sections`. `Minds` shows `New Mind`. `About you` shows the switch `Duragon learns about you`. `Voice models` shows `Install voice models (615 MB download)`.
- **Mind editor.** On `Minds`, choose `New Mind` and wait for the dialog. A dialog named `New Mind` opens over Customize with the textboxes `Name`, `Description …` and `How it thinks …`, and `Cancel`. The last two names carry their hint, so leave out `--exact`. It also has chips under `Start from` (`Staff engineer` to `Blank`) and radios under `Voice` and `How often it speaks up`. Press Escape. `__duragon.app.mindEditor` is `null` and `__duragon.app.customizeOpen` is still `true`. Press Escape again, and Customize closes.
- **Save a mind.** On `Minds`, choose `New Mind`, fill `Name` and `How it thinks`, and press `Control+Enter`. `listRiddlers` gains it, and only the editor closes. Started from the note's mind picker, it also becomes the note's mind (`note-header.md`).
- **Pick a model.** Focus the `Drawing` row with `agent-browser eval '[...document.querySelectorAll(".job .row")].find((b) => b.textContent.includes("Drawing")).focus()'` and press Enter. The dialog `Drawing model` opens, and its combobox `Find a model, or type its name` has focus. Arrows, Home and End move through the listbox `Models`. Type `haiku` and press Enter. The dialog closes, the row has focus, and `drawProvider` and `drawModel` change together in `__duragon.settings.value`.
- **Any name.** Type a name no provider lists, such as `glm-5.2`. The group `Use “glm-5.2” with` offers each ready provider, the current one first. A command added without models lists `default`. An API provider added without models shows the disabled option `Type the name of its model`, and Enter on it does nothing.
- **Thinking.** In the dialog press Tab. The radiogroup `Thinking` (`Auto`, `Low`, `Medium`, `High`, `Highest`) is one tab stop, the arrows pick, and the dialog stays open. One more Tab reaches `Set up another provider`, which opens the Add menu, even from a search that hides `Providers`.
- **Escape.** With the dialog open, Escape closes only that. `__duragon.app.customizeOpen` stays `true`.
- **Save a key.** Choose `Add` (with `--exact`), then the menuitem `OpenRouter`. Its row appears open, with focus in `API key`. Type a key and press Enter. The row shows the key masked, and `(await b.listCredentialStatuses())` has it under `hasStoredKey`.
- **Key that won't save.** Stub the page's backend with `b.setProviderKey = async () => { throw "no space left on device"; }`, and save again. Undo the stub afterwards with `delete b.setProviderKey`. An alert says `The key wasn’t saved: no space left on device. Your old key is still in place.`, and the field keeps the draft.
- **Own provider.** Choose `Add`, then `Something else…`. Pick the radio `A command`, fill `Name` and `Command`, and press Enter. (An `OpenAI` provider with no key and no `Models` can’t be given a job, because the model list doesn’t have it.) Give the `Drawing` job its model `default`, then choose `Remove provider` in its row. The row goes, the row above has focus, and the job is on `claude-cli` with a Claude model.
- **Setting that won't save.** Stub `b.saveSettings = async () => { throw "disk full"; }` and toggle a switch. `delete b.saveSettings` undoes it. It goes back, and a toast says `That change couldn’t be saved.` A provider that can't be removed says so in its row instead, with no toast.
- **Command or address that isn't allowed.** The app asks before a new command, new options or a new address is saved, in a native window, so only the real window proves the question. Make the page's `b.saveSettings` throw `"The change was not allowed"` and change `Claude CLI`'s `Command`. It goes back, and a toast says `That wasn’t allowed, so it wasn’t saved.` A command added under `Something else…` goes away, and `Add` has focus.
- **Keys that can't be read.** Choose `Add` and `OpenRouter` first, because only API providers that are listed show it. Make `b.listCredentialStatuses` return `error` on each API provider, and change a provider's address. The reason stands under `Providers`, and those rows say `Key can’t be read`.
- **Local servers.** Open Customize. The mock's Ollama answers, so its row reads `Ollama 2 models` and the picker offers `qwen3:8b`. Choose `Add`, then `llama.cpp` under `On this computer`. Its row reads `Can’t be reached`, and an alert under `Address` says `Nothing answers at http://localhost:8080/v1.` Change Ollama's `Address`, and its row reads the same until it is changed back. `Check` asks again.
- **Search cleared.** Open Customize, type in `Search settings`, close it, and open it again. The searchbox is empty.
- **Proof.** Take screenshots before and after the toggle, and an `.aria.txt` of the dialog.

## Gotchas

- The searchbox exists only on the `Customize Duragon` page. Because `Control+,` reopens the last page, choose `Customize Duragon` before you search.
- The `Voice` and `Web` switches in the note header have the same role and live outside the dialog. Match the full switch name.
- `About you` is also the name of a textbox on that page. Use `role button` to reach the page.
- While the mind editor sits over Customize, two `Close` buttons exist.
- The dialog has one scroller, `.jobs`' closest `.scroll`. A bare `.scroll` is the side chat's.
- The model list holds only ready providers and the current one, so save a key first to see an API provider in it. Every command line provider counts as ready, because nothing checks that its command exists. A local server counts as ready while it answers.
- Settings in the mock reset on reload.
