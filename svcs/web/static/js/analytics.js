// Analytics functions management page
let currentTab = 'predefined';

function renderAnalytics(container) {
  container.innerHTML = `
    <div class="space-y-6">
      <!-- Tabs -->
      <div class="border-b border-gray-200 dark:border-gray-700">
        <nav class="flex gap-4">
          <button onclick="switchTab('predefined')" id="tab-predefined" class="px-4 py-2 border-b-2 border-blue-600 text-blue-600 font-medium">
            Predefined Functions
          </button>
          <button onclick="switchTab('custom')" id="tab-custom" class="px-4 py-2 border-b-2 border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200">
            Custom Functions
          </button>
        </nav>
      </div>

      <!-- Predefined Functions Tab -->
      <div id="predefined-content" class="space-y-4">
        <div id="predefined-list">
          <div class="animate-pulse space-y-3">
            <div class="bg-gray-200 dark:bg-gray-700 h-24 rounded"></div>
            <div class="bg-gray-200 dark:bg-gray-700 h-24 rounded"></div>
          </div>
        </div>
      </div>

      <!-- Custom Functions Tab -->
      <div id="custom-content" class="hidden space-y-4">
        <div class="flex justify-end">
          <button onclick="showCustomFunctionForm()" class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">
            + Add Custom Function
          </button>
        </div>
        <div id="custom-list">
          <div class="animate-pulse space-y-3">
            <div class="bg-gray-200 dark:bg-gray-700 h-24 rounded"></div>
          </div>
        </div>
      </div>

      <!-- Custom Function Form Modal -->
      <div id="custom-form-modal" class="hidden fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center">
        <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-3xl w-full mx-4 shadow-xl max-h-[90vh] overflow-y-auto">
          <div class="flex justify-between items-center mb-4">
            <h3 class="text-lg font-semibold dark:text-white">Add Custom Function</h3>
            <button onclick="closeCustomForm()" class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">✕</button>
          </div>
          <form id="custom-function-form" onsubmit="submitCustomFunction(event)" class="space-y-4">
            <div>
              <label class="block text-sm font-medium mb-1 dark:text-gray-300">Function Name *</label>
              <input type="text" id="func-name" required pattern="[a-z_][a-z0-9_]*" class="w-full px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white" placeholder="my_custom_forecast" />
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">Lowercase letters, numbers, underscores only</p>
            </div>
            <div>
              <label class="block text-sm font-medium mb-1 dark:text-gray-300">Description</label>
              <textarea id="func-description" rows="2" class="w-full px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white"></textarea>
            </div>
            <div>
              <label class="block text-sm font-medium mb-1 dark:text-gray-300">Python Code *</label>
              <textarea id="func-code" required rows="15" class="w-full px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white font-mono text-sm" placeholder="def forecast(data, params):&#10;    # Your code here&#10;    return result"></textarea>
              <p class="text-xs text-gray-500 dark:text-gray-400 mt-1">Function must accept (data, params) and return a dict</p>
            </div>
            <div class="flex gap-2 justify-end pt-4">
              <button type="button" onclick="closeCustomForm()" class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-800 dark:text-gray-200 rounded hover:bg-gray-300 dark:hover:bg-gray-600">Cancel</button>
              <button type="submit" class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">Save</button>
            </div>
          </form>
        </div>
      </div>
    </div>
  `;
  
  loadFunctions();
}

function switchTab(tab) {
  currentTab = tab;
  
  // Update tab styles
  const predefinedTab = document.getElementById('tab-predefined');
  const customTab = document.getElementById('tab-custom');
  const predefinedContent = document.getElementById('predefined-content');
  const customContent = document.getElementById('custom-content');
  
  if (tab === 'predefined') {
    predefinedTab.className = 'px-4 py-2 border-b-2 border-blue-600 text-blue-600 font-medium';
    customTab.className = 'px-4 py-2 border-b-2 border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200';
    predefinedContent.classList.remove('hidden');
    customContent.classList.add('hidden');
  } else {
    predefinedTab.className = 'px-4 py-2 border-b-2 border-transparent text-gray-500 dark:text-gray-400 hover:text-gray-700 dark:hover:text-gray-200';
    customTab.className = 'px-4 py-2 border-b-2 border-blue-600 text-blue-600 font-medium';
    predefinedContent.classList.add('hidden');
    customContent.classList.remove('hidden');
  }
}

function deleteCustomFunction(id, name) {
  showConfirmModal(
    'Delete Custom Function',
    `Are you sure you want to delete "${name}"? This action cannot be undone.`,
    async () => {
      try {
        const response = await fetch(`/api/v1/analytics/functions/${id}`, { method: 'DELETE' });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        showToast('Function deleted successfully', 'success');
        loadFunctions();
      } catch (error) {
        showToast(`Failed to delete function: ${error.message}`, 'error');
      }
    }
  );
}

async function loadFunctions() {
  try {
    const response = await fetch('/api/v1/analytics/functions');
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = await response.json();
    
    const predefined = data.functions.filter(f => f.is_predefined);
    const custom = data.functions.filter(f => !f.is_predefined);
    
    renderPredefinedFunctions(predefined);
    renderCustomFunctions(custom);
  } catch (error) {
    document.getElementById('predefined-list').innerHTML = `
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <p class="text-red-800 dark:text-red-200">Failed to load functions: ${error.message}</p>
      </div>
    `;
  }
}

function renderPredefinedFunctions(functions) {
  const list = document.getElementById('predefined-list');
  
  if (!functions.length) {
    list.innerHTML = '<p class="text-gray-500 dark:text-gray-400">No predefined functions available</p>';
    return;
  }
  
  list.innerHTML = functions.map(f => `
    <div class="bg-white dark:bg-gray-800 rounded-lg p-4 border dark:border-gray-700">
      <div class="flex justify-between items-start">
        <div class="flex-1">
          <h4 class="font-semibold text-lg dark:text-white">${f.name}</h4>
          <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">${f.description || 'No description'}</p>
          <div class="mt-2 flex gap-2">
            <span class="px-2 py-1 bg-blue-100 text-blue-800 dark:bg-blue-900/30 dark:text-blue-300 rounded text-xs">Predefined</span>
            <span class="px-2 py-1 bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-300 rounded text-xs">${f.language}</span>
          </div>
        </div>
      </div>
    </div>
  `).join('');
}

function showCustomFunctionForm() {
  document.getElementById('custom-form-modal').classList.remove('hidden');
  document.getElementById('custom-function-form').reset();
}

function closeCustomForm() {
  document.getElementById('custom-form-modal').classList.add('hidden');
}

async function submitCustomFunction(event) {
  event.preventDefault();
  
  const name = document.getElementById('func-name').value.trim();
  const description = document.getElementById('func-description').value.trim();
  const code = document.getElementById('func-code').value.trim();
  
  const payload = {
    name,
    description: description || `Custom function: ${name}`,
    language: 'python',
    code
  };
  
  try {
    const response = await fetch('/api/v1/analytics/functions', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(payload)
    });
    
    if (!response.ok) {
      const error = await response.json();
      throw new Error(error.message || `HTTP ${response.status}`);
    }
    
    showToast('Custom function created successfully', 'success');
    closeCustomForm();
    loadFunctions();
    switchTab('custom');
  } catch (error) {
    showToast(`Failed to create function: ${error.message}`, 'error');
  }
}

function renderCustomFunctions(functions) {
  const list = document.getElementById('custom-list');
  
  if (!functions.length) {
    list.innerHTML = '<p class="text-gray-500 dark:text-gray-400">No custom functions. Click "Add Custom Function" to create one.</p>';
    return;
  }
  
  list.innerHTML = functions.map(f => `
    <div class="bg-white dark:bg-gray-800 rounded-lg p-4 border dark:border-gray-700">
      <div class="flex justify-between items-start">
        <div class="flex-1">
          <h4 class="font-semibold text-lg dark:text-white">${f.name}</h4>
          <p class="text-sm text-gray-600 dark:text-gray-400 mt-1">${f.description || 'No description'}</p>
          <div class="mt-2 text-xs text-gray-500 dark:text-gray-400">
            Created: ${new Date(f.created_at).toLocaleDateString()}
          </div>
        </div>
        <div class="flex gap-2">
          <button onclick="deleteCustomFunction(${f.id}, '${f.name}')" class="px-3 py-1 bg-red-600 text-white rounded text-sm hover:bg-red-700">Delete</button>
        </div>
      </div>
    </div>
  `).join('');
}
