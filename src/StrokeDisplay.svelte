<script>
    import { keylistToStroke, strokeToText, textToStroke, strokeToKeydict } from './util.js';
    import { createEventDispatcher } from 'svelte';
    
    const keys = ["#", "S-", "T-", "K-", "P-", "W-", "H-", "R-", "A", "O", "*", "E", "U", "-F", "-R", "-P", "-B", "-L", "-G", "-T", "-S", "-D", "-Z"];
    const replace_map = {"number": "#", "star": "*"};
    
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
    }

    function handle_click(event) {
	// toggle the button's state
	const button = replace_map[event.target.id] || event.target.id;
	state[button] ^= true;

	stroke_changed();
    }
</script>

<style>
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
  
  button {
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

  button.active {
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

<div id="steno-keyboard">
    <button id="number"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["#"]}>
    </button>

    <button id="S-"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["S-"]} class="long-key">
    </button>

    <button id="T-"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["T-"]} class="top-row">
    </button>

    <button id="K-"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["K-"]} class="bottom-row">
    </button>

    <button id="P-"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["P-"]} class="top-row">
    </button>

    <button id="W-"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["W-"]} class="bottom-row">
    </button>

    <button id="H-"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["H-"]} class="top-row">
    </button>

    <button id="R-"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["R-"]} class="bottom-row">
    </button>

    <div id="A-" class="vowel-container">
        <!--note: the button ids don't have the dash, to distinguish them from the div ids!
                (and also to make interfacing with the stroke library easier, where the vowels
                 don't have dashes either)-->
        <button id="A"
            on:click={handle_click}
            on:touchstart|preventDefault={touch_start}
            on:touchmove={touch_move}
            on:touchend={touch_end}
            on:touchcancel={touch_end}
            class:active={state["A"]} class="left-vowel">
        </button>
    </div>

    <div id="O-" class="vowel-container">
        <button id="O"
            on:click={handle_click}
            on:touchstart|preventDefault={touch_start}
            on:touchmove={touch_move}
            on:touchend={touch_end}
            on:touchcancel={touch_end}
            class:active={state["O"]} class="left-vowel">
        </button>
    </div>

    <button id="star"
	on:click={handle_click}
	on:touchstart|preventDefault={touch_start}
	on:touchmove={touch_move}
	on:touchend={touch_end}
	on:touchcancel={touch_end}
	class:active={state["*"]} class="long-key">
    </button>

    <div id="-E" class="vowel-container">
        <button id="E"
            on:click={handle_click}
            on:touchstart|preventDefault={touch_start}
            on:touchmove={touch_move}
            on:touchend={touch_end}
            on:touchcancel={touch_end}
            class:active={state["E"]} class="right-vowel">
        </button>
    </div>

    <div id="-U" class="vowel-container">
        <button id="U"
          on:click={handle_click}
          on:touchstart|preventDefault={touch_start}
          on:touchmove={touch_move}
          on:touchend={touch_end}
          on:touchcancel={touch_end}
          class:active={state["U"]} class="right-vowel">
      </button>
    </div>

    <button id="-F"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["-F"]} class="top-row">
    </button>

    <button id="-R"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["-R"]} class="bottom-row">
    </button>

    <button id="-P"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["-P"]} class="top-row">
    </button>

    <button id="-B"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["-B"]} class="bottom-row">
    </button>

    <button id="-L"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["-L"]} class="top-row">
    </button>

    <button id="-G"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["-G"]} class="bottom-row">
    </button>

    <button id="-T"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["-T"]} class="top-row">
    </button>

    <button id="-S"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["-S"]} class="bottom-row">
    </button>

    <button id="-D"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["-D"]} class="top-row">
    </button>

    <button id="-Z"
        on:click={handle_click}
        on:touchstart|preventDefault={touch_start}
        on:touchmove={touch_move}
        on:touchend={touch_end}
        on:touchcancel={touch_end}
        class:active={state["-Z"]} class="bottom-row">
    </button>
</div>
