<script lang="ts">
  import { onMount } from 'svelte';
  import { JOBS, providerViews } from '../../api/providers';
  import { checkServers, credentialStatuses, servers, settings } from '../../state/settings.svelte';
  import JobRow from './JobRow.svelte';
  import ProvidersPanel from './ProvidersPanel.svelte';
  import WebSearchPanel from './WebSearchPanel.svelte';

  const views = $derived(providerViews(settings.value, credentialStatuses.value, credentialStatuses.error !== null, servers));

  onMount(() => {
    void checkServers();
  });
</script>

<section>
  <h3>Models</h3>
  <p class="sub">Which model does each job, and how hard it thinks.</p>
  <div class="jobs">
    <div class="jobs-head" aria-hidden="true">
      <span>Job</span>
      <span>Model</span>
      <span>Thinking</span>
    </div>
    {#each JOBS as job (job.modelKey)}<JobRow {job} {views} />{/each}
  </div>
  <ProvidersPanel {views} />
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
  .jobs { margin-top: 20px; border: 1px solid var(--rule); border-radius: 12px; }
  .jobs :global(.job + .job) { border-top: 1px solid var(--rule); }
  .jobs-head {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 172px 104px;
    gap: 10px;
    padding: 8px 14px 7px 16px;
    border-bottom: 1px solid var(--rule);
    font-size: 11.5px;
    line-height: 1.3;
    font-weight: 500;
    color: var(--faint);
  }
  @media (max-width: 680px) {
    .jobs-head { display: none; }
  }
</style>
