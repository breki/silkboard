<script lang="ts">
  import { setTheme, themePreference } from "./theme";
</script>

<label class="theme-choice">
  Theme
  <select bind:value={() => $themePreference, setTheme}>
    <option value="system">System</option>
    <option value="light">Light</option>
    <option value="dark">Dark</option>
  </select>
</label>
