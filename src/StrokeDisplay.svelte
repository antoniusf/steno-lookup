<script>
    import { keylistToStroke, strokeToText, textToStroke, strokeToKeydict } from './util.js';
    const keys = ["S-", "T-", "K-", "P-", "W-", "H-", "R-", "A", "O", "star", "E", "U", "-F", "-R", "-P", "-B", "-L", "-G", "-T", "-S", "-D", "-Z"];
    let state = {};
    let stroke = 0;
    let strokeText = "";
    for (const key of keys) {
	state[key] = false;
    }

    //$: stroke = keylistToStroke(
    //    Object.entries(state)
    //	    .filter(([key, state]) => state) // use only keys where state is true
    //	    .map(([key, state]) => (key == "star")? "*" : key) // replace "star" with "*"
    //   );
    $: state = strokeToKeydict(stroke);

    $: stroke = textToStroke(strokeText.toUpperCase());
    //$: strokeText = strokeToText(stroke);

    function handleClick(event) {
	// toggle the button's state
	state[event.target.id] ^= true;
    }
</script>

<style>
  div#steno-keyboard {
    display: grid;
    grid-template-columns: repeat(10, 1fr);
    grid-template-rows: auto auto auto;
    grid-template-areas:
      "S- T- P- H- star -F -P -L -T -D"
      "S- K- W- R- star -R -B -G -S -Z"
      ".  .  A- O-   .  -E -U  .  .  .";
    grid-column-gap: 0.9%;
    grid-row-gap: 2.5%;
    max-width: 70em;
    width: calc(100% - 2em);
    margin: 1em 1em 3em;
  }
  
  button {
    width: 100%;
    padding: 0;
    margin: 0;
    padding-bottom: 120%;
    background-color: #aaa;
    border: none;
    color: white;
    cursor: pointer;
    height: 0;
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
  <button id="S-" on:click={handleClick} class:active={state["S-"]} class="long-key"></button>
  <button id="T-" on:click={handleClick} class:active={state["T-"]} class="top-row"></button>
  <button id="K-" on:click={handleClick} class:active={state["K-"]} class="bottom-row"></button>
  <button id="P-" on:click={handleClick} class:active={state["P-"]} class="top-row"></button>
  <button id="W-" on:click={handleClick} class:active={state["W-"]} class="bottom-row"></button>
  <button id="H-" on:click={handleClick} class:active={state["H-"]} class="top-row"></button>
  <button id="R-" on:click={handleClick} class:active={state["R-"]} class="bottom-row"></button>
  <div id="A-" class="vowel-container">
    <!--note: the button ids don't have the dash, to distinguish them from the div ids!
              (and also to make interfacing with the stroke library easier, where the vowels
               don't have dashes either)
      -->
    <button id="A" on:click={handleClick} class:active={state["A"]} class="left-vowel"></button>
  </div>
  <div id="O-" class="vowel-container">
    <button id="O" on:click={handleClick} class:active={state["O"]} class="left-vowel"></button>
  </div>
  <button id="star" on:click={handleClick} class:active={state["star"]} class="long-key"></button>
  <div id="-E" class="vowel-container">
    <button id="E" on:click={handleClick} class:active={state["E"]} class="right-vowel"></button>
  </div>
  <div id="-U" class="vowel-container">
    <button id="U" on:click={handleClick} class:active={state["U"]} class="right-vowel"></button>
  </div>
  <button id="-F" on:click={handleClick} class:active={state["-F"]} class="top-row"></button>
  <button id="-R" on:click={handleClick} class:active={state["-R"]} class="bottom-row"></button>
  <button id="-P" on:click={handleClick} class:active={state["-P"]} class="top-row"></button>
  <button id="-B" on:click={handleClick} class:active={state["-B"]} class="bottom-row"></button>
  <button id="-L" on:click={handleClick} class:active={state["-L"]} class="top-row"></button>
  <button id="-G" on:click={handleClick} class:active={state["-G"]} class="bottom-row"></button>
  <button id="-T" on:click={handleClick} class:active={state["-T"]} class="top-row"></button>
  <button id="-S" on:click={handleClick} class:active={state["-S"]} class="bottom-row"></button>
  <button id="-D" on:click={handleClick} class:active={state["-D"]} class="top-row"></button>
  <button id="-Z" on:click={handleClick} class:active={state["-Z"]} class="bottom-row"></button>
</div>

<input type="text" bind:value={strokeText} />
