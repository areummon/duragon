<script lang="ts">
  import { onMount } from 'svelte';
  import { JOBS, providerViews } from '../../api/providers';
  import { checkServers, credentialStatuses, servers, settings } from '../../state/settings.svelte';
  import JobRow from './JobRow.svelte';
  import ProvidersPanel from './ProvidersPanel.svelte';
  import WebSearchPanel from './WebSearchPanel.svelte';

  const views = $derived(providerViews(settings.value, credentialStatuses.value, credentialStatuses.error !== null, servers));
  let panel = $state<ProvidersPanel>();

  onMount(() => {
    void checkServers();
  });

  function onsetup() {
    panel?.openAdd();
  }
</script>

<section>
  <h3>Models</h3>
  <p class="sub">Which model does each job, and where it comes from.</p>
  <div class="jobs">
    {#each JOBS as job (job.modelKey)}<JobRow {job} {views} {onsetup} />{/each}
  </div>
  <ProvidersPanel bind:this={panel} {views} />
</section>

<section>
  <h3>Web search</h3>
  <p class="sub">Search engines for HTTP and local models when Web is on.</p>
  <WebSearchPanel />
</section>

<style>
  section { margin-top: 34px; }
  section:first-child { margin-top: 0; }
  h3 { margin: 0; font-size: 17px; line-height: 1.3; font-weight: 500; }
  .sub { margin-top: 3px; font-size: 13.5px; line-height: 1.45; color: var(--faint); }
  .jobs { margin-top: 22px; border: 1px solid var(--rule); border-radius: 12px; }
</style>
