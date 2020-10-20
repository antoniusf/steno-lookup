<!-- This Source Code Form is subject to the terms of the Mozilla Public
   - License, v. 2.0. If a copy of the MPL was not distributed with this
   - file, You can obtain one at https://mozilla.org/MPL/2.0/. -->
<script>
    import { keylistToStroke, strokeToText, textToStroke, strokeToKeydict } from './util.js';
    import { createEventDispatcher } from 'svelte';
    
    const keys = ["#", "S-", "T-", "K-", "P-", "W-", "H-", "R-", "A", "O", "*", "E", "U", "-F", "-R", "-P", "-B", "-L", "-G", "-T", "-S", "-D", "-Z"];
    const replace_map = {"number": "#", "star": "*"};

    let show_keyboard = true;
    
    let state = {};
    for (const key of keys) {
	state[key] = false;
    }

    export let stroke = 0;

    const dispatch = createEventDispatcher();
    $: state = strokeToKeydict(stroke);

    function stroke_changed() {
	dispatch('strokeChanged', {
	    stroke: keylistToStroke(
		Object.entries(state)
		    .filter(([key, state]) => state) // use only keys where state is true
		    .map(([key, state]) => key)
	    )
	});
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
</script>

<style>
    button.keyboard-toggle {
      margin: 0;
      padding: 0.2em 0.4em 0.2em 0.2em;
      background-color: #ab005a;
      color: white;
      border: none;
      cursor: pointer;
      font-size: 0.9rem;
      margin: 0 auto;
      margin-bottom: 1em;
      display: flex;
      align-items: center;
    }

    button.keyboard-toggle:focus {
        outline: 2px solid black;
        outline-offset: 2px;
    }

    button.keyboard-toggle > img {
        vertical-align: middle;
        height: 1.1em;
        margin-right: 0.1em;
    }

    button.keyboard-toggle > span {
        vertical-align: middle;
    }

  div#steno-keyboard {
    display: grid;
    grid-template-columns: repeat(10, 1fr);
    grid-template-rows: auto auto auto auto;
    grid-template-areas:
      "number number number number number number number number number number"
      "S- T- P- H- star -F -P -L -T -D"
      "S- K- W- R- star -R -B -G -S -Z"
      ".  .  A- O-   .  -E -U  .  .  .";
    grid-column-gap: 1.0%;
    grid-row-gap: 2.0%;
    max-width: 70em;
    width: calc(100% - 1.6em);
    margin: 1em 0.8em 10%;
  }

  div#steno-keyboard[hidden] {
      display: none;
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

  div#A- {
    grid-area: A-;
  }

  div#O- {
    grid-area: O-;
  }

  button#star {
    grid-area: star;
  }

  div#-E {
    grid-area: -E;
  }

  div#-U {
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
     class="keyboard-toggle"
     aria-expanded={show_keyboard}
     on:click={(event) => {show_keyboard = !show_keyboard;} }>

     <img src={show_keyboard? "collapse-icon.svg" : "expand-icon.svg"} alt=""/>
     <span>{show_keyboard? "hide steno keyboard" : "show steno keyboard"}</span>
</button>

<!-- I don't think the graphical stroke display is useful for users of
     assistive technology, especially since its functionality is replicated
     one-to-one in the stroke text input field. On the contrary, I think that
     it is probably quite annoying and confusing to tab through, so I've
     decided to just remove it from the accessibility tree for now.
 -->

 <div id="steno-keyboard" hidden={!show_keyboard} role="group" aria-label="steno keyboard">
     <button id="number"
             aria-label="number bar"
             aria-pressed={!!state["#"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno">
     </button>

     <button id="S-"
             aria-label="left S"
             aria-pressed={!!state["S-"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno long-key">
     </button>

     <button id="T-"
             aria-label="left T"
             aria-pressed={!!state["T-"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno top-row">
     </button>

     <button id="K-"
             aria-label="left K"
             aria-pressed={!!state["K-"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno bottom-row">
     </button>

     <button id="P-"
             aria-label="left P"
             aria-pressed={!!state["P-"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno top-row">
     </button>

     <button id="W-"
             aria-label="left W"
             aria-pressed={!!state["W-"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno bottom-row">
     </button>

     <button id="H-"
             aria-label="left H"
             aria-pressed={!!state["H-"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno top-row">
     </button>

     <button id="R-"
             aria-label="left R"
             aria-pressed={!!state["R-"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno bottom-row">
     </button>

     <div id="A-" class="vowel-container">
         <!--note: the button ids don't have the dash, to distinguish them from the div ids!
             (and also to make interfacing with the stroke library easier, where the vowels
             don't have dashes either)-->
             <button id="A"
                     aria-label="A"
                     aria-pressed={!!state["A"]}
                     on:click={handle_click}
                     on:touchstart|preventDefault={touch_start}
                     on:touchmove={touch_move}
                     on:touchend={touch_end}
                     on:touchcancel={touch_end}
                     class="steno left-vowel">
             </button>
     </div>

     <div id="O-" class="vowel-container">
         <button id="O"
                 aria-label="O"
                 aria-pressed={!!state["O"]}
                 on:click={handle_click}
                 on:touchstart|preventDefault={touch_start}
                 on:touchmove={touch_move}
                 on:touchend={touch_end}
                 on:touchcancel={touch_end}
                 class="steno left-vowel">
         </button>
     </div>

     <button id="star"
             aria-label="asterisk"
             aria-pressed={!!state["*"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno long-key">
     </button>

     <div id="-E" class="vowel-container">
         <button id="E"
                 aria-label="E"
                 aria-pressed={!!state["E"]}
                 on:click={handle_click}
                 on:touchstart|preventDefault={touch_start}
                 on:touchmove={touch_move}
                 on:touchend={touch_end}
                 on:touchcancel={touch_end}
                 class="steno right-vowel">
         </button>
     </div>

     <div id="-U" class="vowel-container">
         <button id="U"
                 aria-label="U"
                 aria-pressed={!!state["U"]}
                 on:click={handle_click}
                 on:touchstart|preventDefault={touch_start}
                 on:touchmove={touch_move}
                 on:touchend={touch_end}
                 on:touchcancel={touch_end}
                 class="steno right-vowel">
         </button>
     </div>

     <button id="-F"
             aria-label="right F"
             aria-pressed={!!state["-F"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno top-row">
     </button>

     <button id="-R"
             aria-label="right R"
             aria-pressed={!!state["-R"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno bottom-row">
     </button>

     <button id="-P"
             aria-label="right P"
             aria-pressed={!!state["-P"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno top-row">
     </button>

     <button id="-B"
             aria-label="right B"
             aria-pressed={!!state["-B"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno bottom-row">
     </button>

     <button id="-L"
             aria-label="right L"
             aria-pressed={!!state["-L"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno top-row">
     </button>

     <button id="-G"
             aria-label="right G"
             aria-pressed={!!state["-G"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno bottom-row">
     </button>

     <button id="-T"
             aria-label="right T"
             aria-pressed={!!state["-T"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno top-row">
     </button>

     <button id="-S"
             aria-label="right S"
             aria-pressed={!!state["-S"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno bottom-row">
     </button>

     <button id="-D"
             aria-label="right D"
             aria-pressed={!!state["-D"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno top-row">
     </button>

     <button id="-Z"
             aria-label="right Z"
             aria-pressed={!!state["-Z"]}
             on:click={handle_click}
             on:touchstart|preventDefault={touch_start}
             on:touchmove={touch_move}
             on:touchend={touch_end}
             on:touchcancel={touch_end}
             class="steno bottom-row">
     </button>
 </div>
