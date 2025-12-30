<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  
  interface NotificationData {
    freed_mb: number;
    free_ram_gb: number;
    method: string;
    profile: string;
  }
  
  let notification: NotificationData | null = null;
  let visible = false;
  let unlisten: (() => void) | null = null;
  let timeout: ReturnType<typeof setTimeout> | null = null;
  
  onMount(async () => {
    unlisten = await listen('tmc://show-custom-notification', (event: any) => {
      notification = event.payload as NotificationData;
      visible = true;
      
      if (timeout) clearTimeout(timeout);
      timeout = setTimeout(() => {
        visible = false;
      }, 5000);
    });
  });
  
  onDestroy(() => {
    if (unlisten) unlisten();
    if (timeout) clearTimeout(timeout);
  });
</script>

<style>
  .notification {
    position: fixed;
    bottom: 20px;
    right: 40px;
    background: linear-gradient(135deg, #1a1a2e, #16213e);
    border: 1px solid #0a84ff;
    border-radius: 12px;
    padding: 16px;
    box-shadow: 0 8px 24px rgba(0,0,0,0.4);
    z-index: 10000;
    animation: slideIn 0.3s ease;
    min-width: 280px;
    color: white;
  }
  
  @keyframes slideIn {
    from {
      transform: translateX(100%);
      opacity: 0;
    }
    to {
      transform: translateX(0);
      opacity: 1;
    }
  }
  
  .header {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 12px;
  }
  
  .icon {
    width: 32px;
    height: 32px;
    background: linear-gradient(135deg, #0a84ff, #0066cc);
    border-radius: 8px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-weight: bold;
  }
  
  .title {
    flex: 1;
    font-weight: 600;
    font-size: 14px;
  }
  
  .content {
    display: grid;
    gap: 6px;
  }
  
  .row {
    display: flex;
    justify-content: space-between;
    font-size: 12px;
  }
  
  .label {
    opacity: 0.8;
  }
  
  .value {
    font-weight: 500;
    color: #0a84ff;
  }
  
  .progress {
    height: 4px;
    background: rgba(255,255,255,0.1);
    border-radius: 2px;
    margin-top: 8px;
    overflow: hidden;
  }
  
  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, #0a84ff, #00ff88);
    animation: progressAnim 2s ease;
  }
  
  @keyframes progressAnim {
    from { width: 0%; }
    to { width: 100%; }
  }
</style>

{#if visible && notification}
  <div class="notification">
    <div class="header">
      <div class="icon">✓</div>
      <div class="title">Memory Optimized</div>
    </div>
    
    <div class="content">
      <div class="row">
        <span class="label">RAM Freed:</span>
        <span class="value">{notification.freed_mb.toFixed(1)} MB</span>
      </div>
      
      <div class="row">
        <span class="label">Free RAM:</span>
        <span class="value">{notification.free_ram_gb.toFixed(2)} GB</span>
      </div>
      
      <div class="row">
        <span class="label">Method:</span>
        <span class="value">{notification.method}</span>
      </div>
      
      <div class="row">
        <span class="label">Profile:</span>
        <span class="value">{notification.profile}</span>
      </div>
    </div>
    
    <div class="progress">
      <div class="progress-fill"></div>
    </div>
  </div>
{/if}