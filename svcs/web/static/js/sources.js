// Metric sources configuration page
let editingSourceId = null;

function renderSources(container) {
  container.innerHTML = `
    <div class="space-y-6">
      <!-- Header with Add button -->
      <div class="flex justify-between items-center">
        <h3 class="text-xl font-semibold dark:text-white">Metric Sources</h3>
        <button onclick="showSourceForm()" class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
          + Add Source
        </button>
      </div>

      <!-- Sources List -->
      <div id="sources-list" class="grid gap-4">
        <div class="animate-pulse bg-gray-200 dark:bg-gray-700 h-32 rounded"></div>
      </div>

      <!-- Source Form Modal -->
      <div id="source-form-modal" class="hidden fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center">
        <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-2xl w-full mx-4 shadow-xl max-h-[90vh] overflow-y-auto">
          <div class="flex justify-between items-center mb-4">
            <h3 class="text-lg font-semibold dark:text-white" id="form-title">Add Metric Source</h3>
            <button onclick="closeSourceForm()" class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">✕</button>
          </div>
          <form id="source-form" onsubmit="submitSourceForm(event)" class="space-y-4">
            <div>
              <label class="block text-sm font-medium mb-1 dark:text-gray-300">Name *</label>
              <input type="text" id="input-name" required class="w-full px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
            </div>
            <div>
              <label class="block text-sm font-medium mb-1 dark:text-gray-300">Type *</label>
              <select id="input-type" required onchange="toggleSourceTypeFields()" class="w-full px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white">
                <option value="">Select type...</option>
                <option value="prometheus">Prometheus</option>
                <option value="agent">Agent</option>
              </select>
            </div>
            <div id="prometheus-fields" class="hidden space-y-4">
              <div>
                <label class="block text-sm font-medium mb-1 dark:text-gray-300">Prometheus URL *</label>
                <input type="url" id="input-url" placeholder="http://prometheus:9090" class="w-full px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
              </div>
              <div>
                <label class="block text-sm font-medium mb-1 dark:text-gray-300">Auth Token</label>
                <input type="text" id="input-auth-token" placeholder="Optional Bearer token" class="w-full px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
              </div>
            </div>
            <div class="flex gap-2 justify-end pt-4">
              <button type="button" onclick="closeSourceForm()" class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-800 dark:text-gray-200 rounded hover:bg-gray-300 dark:hover:bg-gray-600">Cancel</button>
              <button type="button" onclick="testConnection()" id="test-btn" class="px-4 py-2 bg-yellow-600 text-white rounded hover:bg-yellow-700">Test Connection</button>
              <button type="submit" class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">Save</button>
            </div>
          </form>
        </div>
      </div>
    </div>
  `;
  
  loadSources();
}

async function loadSources() {
  try {
    const response = await fetch('/api/v1/sources');
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = await response.json();
    renderSourcesList(data.sources || []);
  } catch (error) {
    document.getElementById('sources-list').innerHTML = `
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <p class="text-red-800 dark:text-red-200">Failed to load sources: ${error.message}</p>
        <button onclick="loadSources()" class="mt-2 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700">Retry</button>
      </div>
    `;
  }
}

function editSource(id) {
  showSourceForm(id);
}

function deleteSource(id, name) {
  showConfirmModal(
    'Delete Metric Source',
    `Are you sure you want to delete "${name}"? This action cannot be undone.`,
    async () => {
      try {
        const response = await fetch(`/api/v1/sources/${id}`, { method: 'DELETE' });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        showToast('Source deleted successfully', 'success');
        loadSources();
      } catch (error) {
        showToast(`Failed to delete source: ${error.message}`, 'error');
      }
    }
  );
}

async function testConnection() {
  const type = document.getElementById('input-type').value;
  const url = document.getElementById('input-url').value.trim();
  const authToken = document.getElementById('input-auth-token').value.trim();
  
  if (type !== 'prometheus' || !url) {
    showToast('Please select Prometheus type and enter URL', 'error');
    return;
  }
  
  const btn = document.getElementById('test-btn');
  btn.disabled = true;
  btn.textContent = 'Testing...';
  
  try {
    const params = new URLSearchParams({ url });
    if (authToken) params.append('auth_token', authToken);
    
    const response = await fetch(`/api/v1/sources/test?${params}`);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    
    const result = await response.json();
    showToast(result.message || 'Connection successful', 'success');
  } catch (error) {
    showToast(`Connection test failed: ${error.message}`, 'error');
  } finally {
    btn.disabled = false;
    btn.textContent = 'Test Connection';
  }
}

async function submitSourceForm(event) {
  event.preventDefault();
  
  const name = document.getElementById('input-name').value.trim();
  const type = document.getElementById('input-type').value;
  const url = document.getElementById('input-url').value.trim();
  const authToken = document.getElementById('input-auth-token').value.trim();
  
  const config = {};
  if (type === 'prometheus') {
    config.url = url;
    if (authToken) config.auth_token = authToken;
  }
  
  const payload = { name, source_type: type, config };
  
  try {
    const method = editingSourceId ? 'PUT' : 'POST';
    const endpoint = editingSourceId ? `/api/v1/sources/${editingSourceId}` : '/api/v1/sources';
    
    const response = await fetch(endpoint, {
      method,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });
    
    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || `HTTP ${response.status}`);
    }
    
    showToast(`Source ${editingSourceId ? 'updated' : 'created'} successfully`, 'success');
    closeSourceForm();
    loadSources();
  } catch (error) {
    showToast(`Failed to save source: ${error.message}`, 'error');
  }
}

function renderSourcesList(sources) {
  const list = document.getElementById('sources-list');
  
  if (!sources.length) {
    list.innerHTML = '<p class="text-gray-500 dark:text-gray-400">No metric sources configured. Click "Add Source" to get started.</p>';
    return;
  }
  
  list.innerHTML = sources.map(s => `
    <div class="bg-white dark:bg-gray-800 rounded-lg p-4 border dark:border-gray-700">
      <div class="flex justify-between items-start">
        <div>
          <h4 class="font-semibold text-lg dark:text-white">${s.name}</h4>
          <div class="mt-1 flex items-center gap-2">
            <span class="px-2 py-1 rounded text-xs ${s.source_type === 'prometheus' ? 'bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300' : 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300'}">
              ${s.source_type}
            </span>
            <span class="text-xs text-gray-500 dark:text-gray-400">${s.is_active ? '✓ Active' : '✗ Inactive'}</span>
          </div>
          ${s.config?.url ? `<div class="mt-2 text-sm text-gray-600 dark:text-gray-400">${s.config.url}</div>` : ''}
        </div>
        <div class="flex gap-2">
          <button onclick="editSource(${s.id})" class="px-3 py-1 bg-gray-200 dark:bg-gray-700 text-gray-800 dark:text-gray-200 rounded text-sm hover:bg-gray-300 dark:hover:bg-gray-600">Edit</button>
          <button onclick="deleteSource(${s.id}, '${s.name}')" class="px-3 py-1 bg-red-600 text-white rounded text-sm hover:bg-red-700">Delete</button>
        </div>
      </div>
    </div>
  `).join('');
}

function showSourceForm(sourceId = null) {
  editingSourceId = sourceId;
  const modal = document.getElementById('source-form-modal');
  const form = document.getElementById('source-form');
  const title = document.getElementById('form-title');
  
  form.reset();
  title.textContent = sourceId ? 'Edit Metric Source' : 'Add Metric Source';
  
  if (sourceId) {
    loadSourceForEdit(sourceId);
  }
  
  modal.classList.remove('hidden');
}

function closeSourceForm() {
  document.getElementById('source-form-modal').classList.add('hidden');
  editingSourceId = null;
}

function toggleSourceTypeFields() {
  const type = document.getElementById('input-type').value;
  const promFields = document.getElementById('prometheus-fields');
  const urlInput = document.getElementById('input-url');
  
  if (type === 'prometheus') {
    promFields.classList.remove('hidden');
    urlInput.required = true;
  } else {
    promFields.classList.add('hidden');
    urlInput.required = false;
  }
}

async function loadSourceForEdit(id) {
  try {
    const response = await fetch(`/api/v1/sources/${id}`);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const source = await response.json();
    
    document.getElementById('input-name').value = source.name;
    document.getElementById('input-type').value = source.source_type;
    toggleSourceTypeFields();
    
    if (source.config?.url) {
      document.getElementById('input-url').value = source.config.url;
    }
    if (source.config?.auth_token) {
      document.getElementById('input-auth-token').value = source.config.auth_token;
    }
  } catch (error) {
    showToast(`Failed to load source: ${error.message}`, 'error');
    closeSourceForm();
  }
}
