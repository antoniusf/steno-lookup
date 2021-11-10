<!-- This Source Code Form is subject to the terms of the Mozilla Public
   - License, v. 2.0. If a copy of the MPL was not distributed with this
   - file, You can obtain one at https://mozilla.org/MPL/2.0/. -->
<script>
    import { keylistToStroke, strokeToText, textToStroke, strokeToKeydict } from './util.js';
    import { createEventDispatcher } from 'svelte';

    const keys = ["#", "S-", "T-", "K-", "P-", "W-", "H-", "R-", "A", "O", "*", "E", "U", "-F", "-R", "-P", "-B", "-L", "-G", "-T", "-S", "-D", "-Z"];
    const replace_map = {"number": "#", "star": "*", "new-stroke": "ne", "delete-stroke": "pr"};
    const reverse_replace_map = {"#": "number", "*": "star", "pr": "delete-stroke", "ne": "new-stroke"};

    const steno_keyboard_aria_labels = {
        "#": "number bar",
        "S-": "left S",
        "T-": "left T",
        "K-": "left K",
        "P-": "left P",
        "W-": "left W",
        "H-": "left H",
        "R-": "left R",
        "A": "A",
        "O": "O",
        "*": "asterisk",
        "E": "E",
        "U": "U",
        "-F": "right F",
        "-R": "right R",
        "-P": "right P",
        "-B": "right B",
        "-L": "right L",
        "-G": "right G",
        "-T": "right T",
        "-S": "right S",
        "-D": "right D",
        "-Z": "right Z"
    }

    const steno_keyboard_layout = [
        ["#",  "#",  "#",  "#",  "#", "#",  "#",  "#",  "#",  "#"],
        ["S-", "T-", "P-", "H-", "*", "-F", "-P", "-L", "-T", "-D"],
        ["S-", "K-", "W-", "R-", "*", "-R", "-B", "-G", "-S", "-Z"],
        ["pr", "pr", "A",  "O", null, "E",  "U",  null, "ne", "ne"]
    ];

    // map each key to its (row, column) index
    let steno_keyboard_key_indices = {};
    for (let rowindex = 0; rowindex < steno_keyboard_layout.length; rowindex++) {
        let row = steno_keyboard_layout[rowindex];
        for (let colindex = 0; colindex < row.length; colindex++) {
            let key_name = row[colindex];

            // avoid null keys, and only use the first occurence
            if (key_name && !steno_keyboard_key_indices[key_name]) {
                steno_keyboard_key_indices[key_name] = [rowindex, colindex];
            }
        }
    }

    let steno_keyboard_keymap = {
        "KeyQ": "S-",
        "KeyA": "S-",
        "KeyW": "T-",
        "KeyS": "K-",
        "KeyE": "P-",
        "KeyD": "W-",
        "KeyR": "H-",
        "KeyF": "R-",
        "KeyC": "A",
        "KeyV": "O",
        "KeyT": "*",
        "KeyG": "*",
        "KeyY": "*",
        "KeyH": "*",
        "KeyN": "E",
        "KeyM": "U",
        "KeyU": "-F",
        "KeyJ": "-R",
        "KeyI": "-P",
        "KeyK": "-B",
        "KeyO": "-L",
        "KeyL": "-G",
        "KeyP": "-T",
        "Semicolon": "-S",
        "BracketLeft": "-D",
        "Quote": "-Z"
    }


    const steno_keyboard_buttons = {};
    let steno_keyboard_position = [0, 0];

    let show_keyboard_help = false;

    // the state of each button on the on-screen keyboard
    let state = {};
    // the state of each key on the connected stenotype/keyboard.
    // this distinction is necessary because the full stroke is
    // always accumulated into `state`, which is also what is forwarded
    // to the lookup. however, this requires the buttons in `state` to be
    // held for longer than the actual keyboard keys, since (a) steno keys
    // are accumulated until the stroke is released, and (b) we want to keep
    // showing the stroke even after it has been released.
    let steno_keyboard_physical_state = {};
    for (const key of keys) {
        state[key] = false;
        steno_keyboard_physical_state[key] = false;
    }

    export let strokes = [0];

    $: state = strokeToKeydict(strokes[strokes.length - 1]);

    function stroke_changed() {
        console.log(state);
        const last_stroke = keylistToStroke(
            Object.entries(state)
            .filter(([key, state]) => state) // use only keys where state is true
            .map(([key, state]) => key)
        );

        strokes[strokes.length - 1] = last_stroke;
    }

    // touch handling
    let active_touches = {};
    let last_touch = null;

    function touch_start(event) {

        let touches = event.touches;
        for (let i = 0; i < event.touches.length; i++) {
            let touch = event.touches[i];
            if (!active_touches.hasOwnProperty(touch.identifier)){
                let button = replace_map[event.target.id] || event.target.id;
                state[button] ^= true;

                // to avoid confusion, each swipe gesture can either only turn
                // buttons on or only turn buttons off. so, after toggling the
                // first button, we store it's state so we can set all other
                // buttons this touch hits to the same state.
                let mode = state[button];
                console.log(`new touch with mode ${mode}`);
                active_touches[touch.identifier] = { x: touch.clientX, y: touch.clientY, mode: mode };
            }
        }
        stroke_changed();
    }

    function touch_move(event) {

        let touches = event.changedTouches;
        let changed = false;

        for (let i = 0; i < touches.length; i++) {

            let touch = touches[i];
            let old_touch = active_touches[touch.identifier];
            if (!old_touch) {
                continue;
            }

            // if the swipe is very fast, we may not get
            // an intermediate event for each button that it
            // crosses. so what we do is define a maximum
            // step width and then subdivide the swipe by
            // placing steps along an imaginary line between
            // the start and stop of the swipe.

            // estimate width of a button by window width
            // there's ten buttons, but i'm including a bit of extra buffer
            let step_size = window.innerWidth / 20;
            let delta_x = touch.clientX - old_touch.x;
            let delta_y = touch.clientY - old_touch.y;
            let delta = Math.sqrt(delta_x ** 2 + delta_y ** 2);

            let step_x = delta_x / delta * step_size;
            let step_y = delta_y / delta * step_size;

            while (delta > 0) {
                // start at the end and go backwards; the starting point
                // should have been handled in the previous event handler.
                let x = old_touch.x + delta_x;
                let y = old_touch.y + delta_y;

                delta -= step_size;
                delta_x -= step_x;
                delta_y -= step_y;

                let target = document.elementFromPoint(x, y);
                let button = replace_map[target.id] || target.id;

                if (state.hasOwnProperty(button)) {
                    if (state[button] != old_touch.mode) {
                        state[button] = old_touch.mode;
                        changed = true;
                    }
                }
            }

            // update the stored touch object
            old_touch.x = touch.clientX;
            old_touch.y = touch.clientY;
        }

        if (changed) {
            stroke_changed();
        }
    }

    function touch_end(event) {
        // TODO: do we have to handle this as a move as well?
        let touches = event.changedTouches;
        for (let i = 0; i < touches.length; i++) {
            delete active_touches[touches[i].identifier];
        }
        last_touch = {
            x: touches[touches.length - 1].clientX,
            y: touches[touches.length - 1].clientY,
            target: touches[touches.length - 1].target
        };
        console.log(last_touch);
    }

    function handle_click(event) {

        // avoid double-counting the events. this is necessary since
        // preventDefault appears to be ignored by talkback/firefox for android.
        console.log(`x: ${event.clientX}, y: ${event.clientY}`);
        if (last_touch !== null) {
            if ((event.target == last_touch.target)
                && (last_touch.x == event.clientX)
                && (last_touch.y == event.clientY)) {

                // we should already have handled this event
                // in the touch handler. ignore.
                last_touch = null;
                return;
            }

            last_touch = null;
        }
        // toggle the button's state
        const button = replace_map[event.target.id] || event.target.id;
        state[button] ^= true;

        stroke_changed();
    }

    // keyboard handling
    function steno_key_keydown(event) {
        switch (event.key) {
            case "Down":
            case "ArrowDown":
                try_move_focus([1, 0]);
                event.preventDefault();
                break;

            case "Up":
            case "ArrowUp":
                try_move_focus([-1, 0]);
                event.preventDefault();
                break;

            case "Left":
            case "ArrowLeft":
                try_move_focus([0, -1]);
                break;

            case "Right":
            case "ArrowRight":
                try_move_focus([0, 1]);
                break;

            case "Backspace":
                for (const key of keys) {
                    state[key] = 0;
                }
                stroke_changed();
                break;

            default:
                let mapping = steno_keyboard_keymap[event.code];
                if (mapping) {
                    // set the new key.
                    steno_keyboard_physical_state[mapping] = 1;
                    state[mapping] = 1;
                    stroke_changed();
                }
        }
    }

    function steno_key_keyup(event) {
        let mapping = steno_keyboard_keymap[event.code];
        if (mapping) {
            steno_keyboard_physical_state[mapping] = 0;
            stroke_changed();
        }
    }

    function steno_key_focus(event) {
        let button_name = replace_map[event.target.id] || event.target.id;
        steno_keyboard_position = steno_keyboard_key_indices[button_name];
        console.log(`changed focus to ${steno_keyboard_position} (button name ${button_name})`);
    }

    function try_move_focus(delta) {

        const current_key_name = steno_keyboard_layout[steno_keyboard_position[0]][steno_keyboard_position[1]];
        let new_position = steno_keyboard_position.slice();
        let key_name = current_key_name;

        while (!key_name || key_name == current_key_name) {
            // move until the key changes, or we hit another valid key
            // this is for skipping the gap between the vowels,
            // as well as for the long keys

            new_position = [
                new_position[0] + delta[0],
                new_position[1] + delta[1]
            ];

            if (new_position[0] < 0 || new_position[0] >= steno_keyboard_layout.length
                || new_position[1] < 0 || new_position[1] >= steno_keyboard_layout[0].length) {
                // no changes made
                return;
            }

            key_name = steno_keyboard_layout[new_position[0]][new_position[1]];
        }

        steno_keyboard_buttons[key_name].focus();
        steno_keyboard_position = new_position;
        console.log(steno_keyboard_position);
    }
</script>

<style>
    button.keyboard-help-toggle {
        margin: 0.7em auto 0;
        padding: 0.2em 0.4em 0.2em 0.2em;
        background-color: white;
        color: #333;
        border: none;
        cursor: pointer;
        display: flex;
        align-items: center;
    }

    button/*.keyboard-toggle*/:focus {
        outline: 2px solid black;
        outline-offset: 2px;
    }

    button.keyboard-help-toggle > img {
        vertical-align: middle;
        height: 1.1em;
        margin-right: 0.1em;
        filter: brightness(0.2);
    }

    button.keyboard-help-toggle > span {
        vertical-align: middle;
    }

    p.steno-keyboard-hint {
        margin: 0.5rem 0;
        font-size: 0.9rem;
        color: #444;
        text-align: left;
        padding: 0em 1em;
    }

    div#steno-keyboard-hint-container {
        margin: 0.5em 1em 1em;
    }

    div#steno-keyboard-hint-container[hidden] {
        display:none;
    }

    div#steno-keyboard {
        display: grid;
        grid-template-columns: repeat(10, 1fr);
        grid-template-rows: auto auto auto auto;
        grid-template-areas:
            "number number number number number number number number number number"
        "S- T- P- H- star -F -P -L -T -D"
        "S- K- W- R- star -R -B -G -S -Z"
        "pr pr A- O-   .  -E -U  . ne ne";
        grid-column-gap: 1.0%;
        grid-row-gap: 2.0%;
        max-width: 70em;
        width: calc(100% - 1.6em);
        margin: 0.5em 0.8em 10%;
    }

    button.steno-action {
        width: fit-content;
        height: fit-content;
        padding: 0.2em 0.4em;
        margin: 0;
        background-color: #aaa;
        border: none;
        color: black;
        cursor: pointer;
        align-self: center;
        justify-self: center;
        position: relative;
        top: 30%;
    }

    button.steno-action:focus {
        outline: 2px solid #e50078;
        outline-offset: 2px;
        z-index: 10; /* make sure the focus ring is visible */
    }

    button#delete-stroke {
        grid-area: pr;
    }

    button#new-stroke {
        grid-area: ne;
    }

    button.steno {
        width: 100%;
        padding: 0;
        margin: 0;
        padding-bottom: 110%;
        background-color: #aaa;
        border: none;
        color: white;
        cursor: pointer;
        height: 0;
    }

    button.steno:focus {
        outline: 2px solid #e50078;
        outline-offset: 2px;
        z-index: 10; /* make sure the focus ring is visible */
    }

    button#number {
        grid-area: number;
        padding-bottom: 4%;
    }

    button[aria-pressed="true"] {
        background-color: #000;
    }

    button.top-row {
        border-radius: 3% / 2%;
    }

    button.bottom-row,
    button.right-vowel,
    button.left-vowel {
        border-radius: 3% / 2%;
        border-bottom-left-radius: 50% 20%;
        border-bottom-right-radius: 50% 20%;
    }

    button.long-key {
        border-radius: 3% / 1%;
        border-bottom-left-radius: 50% 10%;
        border-bottom-right-radius: 50% 10%;
        height: 100%;
        padding: 0;
    }

    div.vowel-container {
        width: 100%;
        height: auto;
        padding: 0;
        margin: 0;
    }

    button.left-vowel {
        position: relative;
        left: 25%;
        top: 30%;
        vertical-align: text-bottom;
    }

    button.right-vowel {
        position: relative;
        left: -25%;
        top: 30%;
        vertical-align: text-bottom;
    }

    button#S- {
        grid-area: S-;
    }

    button#T- {
        grid-area: T-;
    }

    button#K- {
        grid-area: K-;
    }

    button#P- {
        grid-area: P-;
    }

    button#W- {
        grid-area: W-;
    }

    button#H- {
        grid-area: H-;
    }

    button#R- {
        grid-area: R-;
    }

    div#A-, button#A {
        grid-area: A-;
    }

    div#O-, button#O {
        grid-area: O-;
    }

    button#star {
        grid-area: star;
    }

    div#-E, button#E {
        grid-area: -E;
    }

    div#-U, button#U {
        grid-area: -U;
    }

    button#-F {
        grid-area: -F;
    }

    button#-R {
        grid-area: -R;
    }

    button#-P {
        grid-area: -P;
    }

    button#-B {
        grid-area: -B;
    }

    button#-L {
        grid-area: -L;
    }

    button#-G {
        grid-area: -G;
    }

    button#-T {
        grid-area: -T;
    }

    button#-S {
        grid-area: -S;
    }

    button#-D {
        grid-area: -D;
    }

    button#-Z {
        grid-area: -Z;
    }
</style>

<button
          class="keyboard-help-toggle"
          aria-expanded={show_keyboard_help}
          on:click={(event) => {show_keyboard_help = !show_keyboard_help;} }>

          <img src={show_keyboard_help? "collapse-icon.svg" : "expand-icon.svg"} alt=""/>
          <span>virtual stenotype help</span>
</button>

<div id="steno-keyboard-hint-container" hidden={!show_keyboard_help}>

    <p class="steno-keyboard-hint">The virtual stenotype always shows the last stroke of the search box. Use "add" to add a new, empty stroke, and "delete" to remove the current stroke and go back to the previous one.</p>
    <p class="steno-keyboard-hint"><b>Mouse:</b> Click keys to toggle them.</p>
    <p class="steno-keyboard-hint"><b>Touch:</b> Touch or swipe over keys to toggle them.</p>
    <p class="steno-keyboard-hint"><b>Keyboard:</b> Use arrow keys to move between buttons, and space or return to toggle.</p>
    <p class="steno-keyboard-hint"><b>Keyboard (direct input):</b> Move focus to any button, then use a your keyboard as a qwerty stenotype. Pressing a key will add it to the chord; to clear the entire chord, use backspace.</p>

</div>

<div id="steno-keyboard" role="group" aria-label="steno keyboard">
    {#each keys as key_name}
        <button id={reverse_replace_map[key_name] || key_name}
                aria-label={steno_keyboard_aria_labels[key_name]}
                aria-pressed={!!state[key_name]}
                on:click={handle_click}
                on:touchstart|preventDefault={touch_start}
                on:touchmove={touch_move}
                on:touchend={touch_end}
                on:touchcancel={touch_end}
                on:keydown={steno_key_keydown}
                on:keyup={steno_key_keyup}
                on:focus={steno_key_focus}
                tabindex={(steno_keyboard_layout[steno_keyboard_position[0]][steno_keyboard_position[1]] == key_name)? 0 : -1}
                bind:this={steno_keyboard_buttons[key_name]}

                class:long-key={(key_name == "S-") || (key_name == "*")}
                class:top-row={steno_keyboard_key_indices[key_name][0] == 1}
                class:bottom-row={steno_keyboard_key_indices[key_name][0] == 2}
                class:left-vowel={key_name == "A" || key_name == "O"}
                class:right-vowel={key_name == "E" || key_name == "U"}
                class="steno">
        </button>
    {/each}
    <button id="delete-stroke"
                on:click={(event) => {
                    strokes.pop();
                    if (strokes.length == 0) {
                        strokes.push(0);
                    }
                    state = strokeToKeydict(strokes[strokes.length - 1]);
                    strokes = strokes; // make sure update gets reacted on internally
                    stroke_changed(); // emit change event
                }}
                on:keydown={steno_key_keydown}
                on:keyup={steno_key_keyup}
                on:focus={steno_key_focus}
            tabindex={(steno_keyboard_layout[steno_keyboard_position[0]][steno_keyboard_position[1]] == "pr")? 0 : -1}
            bind:this={steno_keyboard_buttons["pr"]}
            class="steno-action">
        delete</button>
    <button id="new-stroke"
                on:click={(event) => {
                    if (strokes[strokes.length - 1] != 0) {
                        strokes.push(0);
                        state = strokeToKeydict(strokes[strokes.length - 1]);
                        strokes = strokes; // make sure update gets reacted on internally
                        stroke_changed(); // emit change event
                    }
                }}
                on:keydown={steno_key_keydown}
                on:keyup={steno_key_keyup}
                on:focus={steno_key_focus}
            tabindex={(steno_keyboard_layout[steno_keyboard_position[0]][steno_keyboard_position[1]] == "ne")? 0 : -1}
            bind:this={steno_keyboard_buttons["ne"]}
            class="steno-action">
        add</button>
</div>
