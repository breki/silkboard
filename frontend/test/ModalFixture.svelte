<script>
  import Modal from "../src/Modal.svelte";
  let open = $state(true);
</script>

<button onclick={() => (open = true)}>Open choices</button>
<Modal {open} title="Test choices" onclose={() => (open = false)}>
  <input aria-label="Search choices" data-initial-focus />
  <button disabled>Unavailable</button>
  <button>Last choice</button>
</Modal>
