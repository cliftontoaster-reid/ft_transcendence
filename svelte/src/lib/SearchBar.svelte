<script lang="ts">
  let { searchQuery = $bindable('') }: { searchQuery: string } = $props();
</script>

<div class="search-container">
  <div class="search-field">
    <input
      type="text"
      placeholder="Search colors or summaries..."
      bind:value={searchQuery}
    />
    <svg
      class="search-icon"
      xmlns="http://www.w3.org/2000/svg"
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <circle cx="11" cy="11" r="7"></circle>
      <path d="m20 20-4-4"></path>
    </svg>
  </div>
</div>

<style>
  .search-container {
    margin: 20px auto;
    max-width: 400px;
    width: 100%;
  }

  .search-field {
    position: relative;
  }

  input {
    width: 100%;
    padding: 12px 44px 12px 16px;
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

  .search-icon {
    position: absolute;
    top: 50%;
    right: 14px;
    width: 18px;
    height: 18px;
    color: rgba(255, 255, 255, 0.65);
    transform: translateY(-50%);
    pointer-events: none;
  }
</style>