<script lang="ts">
  import { invoke, isTauri } from "@tauri-apps/api/core";

  // The same frontend also runs as a static page (`cli build`), with no backend.
  const coreVersion: Promise<string> = isTauri()
    ? invoke<string>("core_version")
    : Promise.resolve("static page");
</script>

<main>
  <h1>dialogue-maker</h1>
  <p>Scaffold. Opening a project arrives with M3.</p>
  {#await coreVersion then version}
    <p class="muted">core {version}</p>
  {/await}
</main>

<style>
  main {
    max-width: 40rem;
    margin: 4rem auto;
    padding: 0 1rem;
  }
  .muted {
    color: var(--muted);
  }
</style>
