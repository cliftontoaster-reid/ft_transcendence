<script lang="ts">
  let { searchQuery = $bindable('') }: { searchQuery: string } = $props();
</script>

<div class="search-container">
  <input 
    type="text" 
    placeholder="Search colors or summaries..." 
    bind:value={searchQuery}
  />
</div>

<style>
  .search-container {
    margin: 20px auto;
    max-width: 400px;
    width: 100%;
  }

  input {
    width: 100%;
    padding: 12px 16px;
    font-size: 16px;
    background: rgba(255, 255, 255, 0.1);
    border: 1px solid rgba(255, 255, 255, 0.2);
    border-radius: 8px;
    color: white;
    outline: none;
    box-sizing: border-box;
    transition: border-color 0.2s ease, background 0.2s ease;
  }

  input::placeholder {
    color: rgba(255, 255, 255, 0.5);
  }

  input:focus {
    background: rgba(255, 255, 255, 0.15);
    border-color: rgba(255, 255, 255, 0.5);
  }
</style>