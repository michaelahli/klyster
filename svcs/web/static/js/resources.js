// Resource groups management page
let editingGroupId = null;
let selectedGroupId = null;

function renderResources(container) {
  container.innerHTML = `
    <div class="space-y-6">
      <!-- Header with Add button -->
      <div class="flex justify-between items-center">
        <h3 class="text-xl font-semibold dark:text-white">Resource Groups</h3>
        <button onclick="showGroupForm()" class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
          + Add Group
        </button>
      </div>

      <!-- Groups List -->
      <div id="groups-list" class="grid gap-4">
        <div class="animate-pulse bg-gray-200 dark:bg-gray-700 h-32 rounded"></div>
      </div>

      <!-- Group Form Modal -->
      <div id="group-form-modal" class="hidden fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center">
        <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-md w-full mx-4 shadow-xl">
          <div class="flex justify-between items-center mb-4">
            <h3 class="text-lg font-semibold dark:text-white" id="group-form-title">Add Resource Group</h3>
            <button onclick="closeGroupForm()" class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">✕</button>
          </div>
          <form id="group-form" onsubmit="submitGroupForm(event)" class="space-y-4">
            <div>
              <label class="block text-sm font-medium mb-1 dark:text-gray-300">Name *</label>
              <input type="text" id="group-input-name" required class="w-full px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
            </div>
            <div>
              <label class="block text-sm font-medium mb-1 dark:text-gray-300">Description</label>
              <textarea id="group-input-description" rows="3" class="w-full px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white"></textarea>
            </div>
            <div class="flex gap-2 justify-end pt-4">
              <button type="button" onclick="closeGroupForm()" class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-800 dark:text-gray-200 rounded hover:bg-gray-300 dark:hover:bg-gray-600">Cancel</button>
              <button type="submit" class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">Save</button>
            </div>
          </form>
        </div>
      </div>

      <!-- Group Detail Modal -->
      <div id="group-detail-modal" class="hidden fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center">
        <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-4xl w-full mx-4 shadow-xl max-h-[90vh] overflow-y-auto">
          <div class="flex justify-between items-center mb-4">
            <h3 class="text-lg font-semibold dark:text-white" id="detail-group-name">Group Details</h3>
            <button onclick="closeGroupDetail()" class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">✕</button>
          </div>
          <div id="group-detail-content"></div>
        </div>
      </div>
    </div>
  `;
  
  loadGroups();
}

async function loadGroups() {
  try {
    const response = await fetch('/api/v1/resource-groups');
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = await response.json();
    renderGroupsList(data.groups || []);
  } catch (error) {
    document.getElementById('groups-list').innerHTML = `
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <p class="text-red-800 dark:text-red-200">Failed to load resource groups: ${error.message}</p>
        <button onclick="loadGroups()" class="mt-2 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700">Retry</button>
      </div>
    `;
  }
}

function editGroup(id) {
  showGroupForm(id);
}

function deleteGroup(id, name) {
  showConfirmModal(
    'Delete Resource Group',
    `Are you sure you want to delete "${name}"? This will also delete all associated resources and scaling targets.`,
    async () => {
      try {
        const response = await fetch(`/api/v1/resource-groups/${id}`, { method: 'DELETE' });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        showToast('Group deleted successfully', 'success');
        loadGroups();
      } catch (error) {
        showToast(`Failed to delete group: ${error.message}`, 'error');
      }
    }
  );
}

async function viewGroupDetail(id) {
  selectedGroupId = id;
  const modal = document.getElementById('group-detail-modal');
  const content = document.getElementById('group-detail-content');
  
  content.innerHTML = '<div class="animate-pulse space-y-3"><div class="bg-gray-200 dark:bg-gray-700 h-10 rounded"></div></div>';
  modal.classList.remove('hidden');
  
  try {
    const response = await fetch(`/api/v1/resource-groups/${id}`);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = await response.json();
    
    document.getElementById('detail-group-name').textContent = data.group.name;
    renderGroupDetail(data);
  } catch (error) {
    content.innerHTML = `
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <p class="text-red-800 dark:text-red-200">Failed to load details: ${error.message}</p>
      </div>
    `;
  }
}

function closeGroupDetail() {
  document.getElementById('group-detail-modal').classList.add('hidden');
  selectedGroupId = null;
}

function renderGroupDetail(data) {
  const content = document.getElementById('group-detail-content');
  const { group, resources, scaling_targets } = data;
  
  content.innerHTML = `
    <div class="space-y-6">
      <!-- Group Info -->
      <div class="border-b dark:border-gray-700 pb-4">
        <div class="grid grid-cols-2 gap-4 text-sm">
          <div>
            <span class="text-gray-500 dark:text-gray-400">Provider:</span>
            <span class="ml-2 dark:text-gray-300">${group.provider_type}</span>
          </div>
          <div>
            <span class="text-gray-500 dark:text-gray-400">Created:</span>
            <span class="ml-2 dark:text-gray-300">${new Date(group.created_at).toLocaleDateString()}</span>
          </div>
        </div>
        ${group.description ? `<p class="mt-2 text-sm text-gray-600 dark:text-gray-400">${group.description}</p>` : ''}
      </div>
      
      <!-- Resources -->
      <div>
        <h4 class="font-semibold mb-2 dark:text-white">Resources (${resources?.length || 0})</h4>
        ${resources?.length ? `
          <div class="space-y-2">
            ${resources.map(r => `
              <div class="bg-gray-50 dark:bg-gray-900 rounded p-3 text-sm">
                <div class="font-medium dark:text-white">${r.name}</div>
                <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">Kind: ${r.kind}</div>
              </div>
            `).join('')}
          </div>
        ` : '<p class="text-gray-500 dark:text-gray-400 text-sm">No resources in this group</p>'}
      </div>
      
      <!-- Scaling Targets -->
      <div>
        <h4 class="font-semibold mb-2 dark:text-white">Scaling Targets (${scaling_targets?.length || 0})</h4>
        ${scaling_targets?.length ? `
          <div class="space-y-2">
            ${scaling_targets.map(t => `
              <div class="bg-gray-50 dark:bg-gray-900 rounded p-3 text-sm">
                <div class="font-medium dark:text-white">${t.metric_name}</div>
                <div class="text-xs text-gray-500 dark:text-gray-400 mt-1">
                  Min: ${t.min_replicas} | Max: ${t.max_replicas} | Target: ${t.target_value}
                </div>
              </div>
            `).join('')}
          </div>
        ` : '<p class="text-gray-500 dark:text-gray-400 text-sm">No scaling targets configured</p>'}
      </div>
    </div>
  `;
}

function renderGroupsList(groups) {
  const list = document.getElementById('groups-list');
  
  if (!groups.length) {
    list.innerHTML = '<p class="text-gray-500 dark:text-gray-400">No resource groups configured. Click "Add Group" to get started.</p>';
    return;
  }
  
  list.innerHTML = groups.map(g => `
    <div class="bg-white dark:bg-gray-800 rounded-lg p-4 border dark:border-gray-700">
      <div class="flex justify-between items-start">
        <div class="flex-1" onclick="viewGroupDetail(${g.id})" style="cursor: pointer;">
          <h4 class="font-semibold text-lg dark:text-white">${g.name}</h4>
          ${g.description ? `<p class="text-sm text-gray-600 dark:text-gray-400 mt-1">${g.description}</p>` : ''}
          <div class="mt-2 text-xs text-gray-500 dark:text-gray-400">
            Provider: ${g.provider_type} • Created: ${new Date(g.created_at).toLocaleDateString()}
          </div>
        </div>
        <div class="flex gap-2">
          <button onclick="editGroup(${g.id})" class="px-3 py-1 bg-gray-200 dark:bg-gray-700 text-gray-800 dark:text-gray-200 rounded text-sm hover:bg-gray-300 dark:hover:bg-gray-600">Edit</button>
          <button onclick="deleteGroup(${g.id}, '${g.name}')" class="px-3 py-1 bg-red-600 text-white rounded text-sm hover:bg-red-700">Delete</button>
        </div>
      </div>
    </div>
  `).join('');
}

function showGroupForm(groupId = null) {
  editingGroupId = groupId;
  const modal = document.getElementById('group-form-modal');
  const form = document.getElementById('group-form');
  const title = document.getElementById('group-form-title');
  
  form.reset();
  title.textContent = groupId ? 'Edit Resource Group' : 'Add Resource Group';
  
  if (groupId) {
    loadGroupForEdit(groupId);
  }
  
  modal.classList.remove('hidden');
}

function closeGroupForm() {
  document.getElementById('group-form-modal').classList.add('hidden');
  editingGroupId = null;
}

async function loadGroupForEdit(id) {
  try {
    const response = await fetch(`/api/v1/resource-groups/${id}`);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = await response.json();
    
    document.getElementById('group-input-name').value = data.group.name;
    document.getElementById('group-input-description').value = data.group.description || '';
  } catch (error) {
    showToast(`Failed to load group: ${error.message}`, 'error');
    closeGroupForm();
  }
}

async function submitGroupForm(event) {
  event.preventDefault();
  
  const name = document.getElementById('group-input-name').value.trim();
  const description = document.getElementById('group-input-description').value.trim();
  
  const payload = {
    name,
    description,
    provider_type: 'kubernetes',
    provider_config: {}
  };
  
  try {
    const method = editingGroupId ? 'PUT' : 'POST';
    const endpoint = editingGroupId ? `/api/v1/resource-groups/${editingGroupId}` : '/api/v1/resource-groups';
    
    const response = await fetch(endpoint, {
      method,
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });
    
    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || `HTTP ${response.status}`);
    }
    
    showToast(`Group ${editingGroupId ? 'updated' : 'created'} successfully`, 'success');
    closeGroupForm();
    loadGroups();
  } catch (error) {
    showToast(`Failed to save group: ${error.message}`, 'error');
  }
}
