<script>
import { invoke } from '@tauri-apps/api/tauri'

let name = ''
let greetMsg = ''

async function greet() {
  greetMsg = await invoke('greet', { name })
}
</script>

<div>
<form class="row" id="greet-form">
  <input id="greet-input" on:submit="{greet}" placeholder="autonomi website address" bind:value="{name}" />
  <p>{greetMsg}</p>
</form>
</div>
