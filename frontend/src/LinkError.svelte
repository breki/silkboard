<script lang="ts">
  import { navigateUrl } from "./link-state";
</script>

<section class="notice" aria-label="Invalid view link">
  <p role="alert">This link contains invalid or unsupported view settings.</p>
  <a
    href={window.location.pathname}
    onclick={(event) => {
      event.preventDefault();
      navigateUrl(window.location.pathname);
    }}>Reset view</a
  >
</section>
