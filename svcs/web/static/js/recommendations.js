// Recommendations page
let selectedRecommendations = new Set();
let currentFilter = 'pending';

function renderRecommendations(container) {
  container.innerHTML = `
    <div class="space-y-6">
      <!-- Filters and Bulk Actions -->
      <div class="bg-white dark:bg-gray-800 rounded-lg p-4">
        <div class="flex flex-wrap justify-between items-center gap-4">
          <div class="flex gap-2">
            <button onclick="filterRecommendations('pending')" id="filter-pending" class="px-4 py-2 rounded bg-blue-600 text-white">
              Pending
            </button>
            <button onclick="filterRecommendations('approved')" id="filter-approved" class="px-4 py-2 rounded bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300">
              Approved
            </button>
            <button onclick="filterRecommendations('dismissed')" id="filter-dismissed" class="px-4 py-2 rounded bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300">
              Dismissed
            </button>
          </div>
          <div id="bulk-actions" class="hidden flex gap-2">
            <button onclick="bulkApprove()" class="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700">
              Approve Selected
            </button>
            <button onclick="bulkDismiss()" class="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700">
              Dismiss Selected
            </button>
          </div>
        </div>
      </div>

      <!-- Recommendations Table -->
      <div class="bg-white dark:bg-gray-800 rounded-lg p-6 overflow-x-auto">
        <div id="recommendations-table">
          <div class="animate-pulse space-y-3">
            <div class="bg-gray-200 dark:bg-gray-700 h-10 rounded"></div>
            <div class="bg-gray-200 dark:bg-gray-700 h-10 rounded"></div>
          </div>
        </div>
      </div>

      <!-- Pagination -->
      <div id="pagination" class="flex justify-center gap-2"></div>
    </div>
  `;
  
  loadRecommendations();
}

function bulkDismiss() {
  if (selectedRecommendations.size === 0) return;
  
  const ids = Array.from(selectedRecommendations);
  
  showConfirmModal(
    'Dismiss Multiple Recommendations',
    `Are you sure you want to dismiss ${ids.length} recommendation${ids.length > 1 ? 's' : ''}?`,
    async () => {
      let succeeded = 0;
      let failed = 0;
      
      for (const id of ids) {
        try {
          const response = await fetch(`/api/v1/recommendations/${id}/dismiss`, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' }
          });
          if (response.ok) succeeded++;
          else failed++;
        } catch {
          failed++;
        }
      }
      
      showToast(`Dismissed ${succeeded}/${ids.length} recommendations${failed > 0 ? `, ${failed} failed` : ''}`, succeeded > 0 ? 'success' : 'error');
      selectedRecommendations.clear();
      loadRecommendations();
    }
  );
}

function filterRecommendations(status) {
  currentFilter = status;
  selectedRecommendations.clear();
  
  // Update button styles
  ['pending', 'approved', 'dismissed'].forEach(s => {
    const btn = document.getElementById(`filter-${s}`);
    if (s === status) {
      btn.className = 'px-4 py-2 rounded bg-blue-600 text-white';
    } else {
      btn.className = 'px-4 py-2 rounded bg-gray-200 dark:bg-gray-700 text-gray-700 dark:text-gray-300';
    }
  });
  
  loadRecommendations();
}

async function loadRecommendations() {
  const url = currentFilter === 'pending' 
    ? '/api/v1/recommendations/pending'
    : `/api/v1/recommendations?status=${currentFilter}`;
  
  try {
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = await response.json();
    renderRecommendationsTable(data.recommendations || []);
  } catch (error) {
    document.getElementById('recommendations-table').innerHTML = `
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <p class="text-red-800 dark:text-red-200">Failed to load recommendations: ${error.message}</p>
        <button onclick="loadRecommendations()" class="mt-2 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700">Retry</button>
      </div>
    `;
  }
}

async function approveRecommendation(id) {
  try {
    const response = await fetch(`/api/v1/recommendations/${id}/approve`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' }
    });
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    showToast('Recommendation approved', 'success');
    loadRecommendations();
  } catch (error) {
    showToast(`Failed to approve: ${error.message}`, 'error');
  }
}

function dismissRecommendation(id) {
  showConfirmModal(
    'Dismiss Recommendation',
    'Are you sure you want to dismiss this recommendation?',
    async () => {
      try {
        const response = await fetch(`/api/v1/recommendations/${id}/dismiss`, {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' }
        });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        showToast('Recommendation dismissed', 'success');
        loadRecommendations();
      } catch (error) {
        showToast(`Failed to dismiss: ${error.message}`, 'error');
      }
    }
  );
}

async function bulkApprove() {
  if (selectedRecommendations.size === 0) return;
  
  const ids = Array.from(selectedRecommendations);
  let succeeded = 0;
  let failed = 0;
  
  for (const id of ids) {
    try {
      const response = await fetch(`/api/v1/recommendations/${id}/approve`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' }
      });
      if (response.ok) succeeded++;
      else failed++;
    } catch {
      failed++;
    }
  }
  
  showToast(`Approved ${succeeded}/${ids.length} recommendations${failed > 0 ? `, ${failed} failed` : ''}`, succeeded > 0 ? 'success' : 'error');
  selectedRecommendations.clear();
  loadRecommendations();
}

function renderRecommendationsTable(recommendations) {
  const table = document.getElementById('recommendations-table');
  
  if (!recommendations.length) {
    table.innerHTML = '<p class="text-gray-500 dark:text-gray-400">No recommendations found</p>';
    return;
  }
  
  const canAct = currentFilter === 'pending';
  
  table.innerHTML = `
    <table class="min-w-full divide-y divide-gray-200 dark:divide-gray-700">
      <thead>
        <tr class="text-left text-sm font-medium text-gray-500 dark:text-gray-400">
          ${canAct ? '<th class="pb-3 pr-3"><input type="checkbox" onclick="toggleSelectAll(this)" /></th>' : ''}
          <th class="pb-3 pr-3">Resource Group</th>
          <th class="pb-3 pr-3">Action</th>
          <th class="pb-3 pr-3">Current</th>
          <th class="pb-3 pr-3">Recommended</th>
          <th class="pb-3 pr-3">Confidence</th>
          <th class="pb-3 pr-3">Reason</th>
          ${canAct ? '<th class="pb-3">Actions</th>' : ''}
        </tr>
      </thead>
      <tbody class="divide-y divide-gray-200 dark:divide-gray-700">
        ${recommendations.map(r => renderRecommendationRow(r, canAct)).join('')}
      </tbody>
    </table>
  `;
  
  updateBulkActionsVisibility();
}

function renderRecommendationRow(rec, canAct) {
  const actionBadge = rec.action === 'scale_up' ? 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-300' : 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-300';
  const confidenceColor = rec.confidence_score >= 80 ? 'text-green-600' : rec.confidence_score >= 60 ? 'text-yellow-600' : 'text-red-600';
  
  return `
    <tr class="text-sm dark:text-gray-300">
      ${canAct ? `<td class="py-3 pr-3"><input type="checkbox" onchange="toggleRecommendation(${rec.id}, this.checked)" /></td>` : ''}
      <td class="py-3 pr-3 font-medium">${rec.resource_group_name || 'N/A'}</td>
      <td class="py-3 pr-3"><span class="px-2 py-1 rounded text-xs ${actionBadge}">${rec.action}</span></td>
      <td class="py-3 pr-3">${rec.current_replicas || 0}</td>
      <td class="py-3 pr-3">${rec.recommended_replicas || 0}</td>
      <td class="py-3 pr-3 ${confidenceColor} font-medium">${rec.confidence_score?.toFixed(1) || 'N/A'}%</td>
      <td class="py-3 pr-3 text-xs text-gray-500 dark:text-gray-400">${rec.reason || 'N/A'}</td>
      ${canAct ? `
        <td class="py-3 flex gap-2">
          <button onclick="approveRecommendation(${rec.id})" class="px-3 py-1 bg-green-600 text-white rounded text-xs hover:bg-green-700">Approve</button>
          <button onclick="dismissRecommendation(${rec.id})" class="px-3 py-1 bg-red-600 text-white rounded text-xs hover:bg-red-700">Dismiss</button>
        </td>
      ` : ''}
    </tr>
  `;
}

function toggleSelectAll(checkbox) {
  const checkboxes = document.querySelectorAll('tbody input[type="checkbox"]');
  checkboxes.forEach(cb => {
    cb.checked = checkbox.checked;
    const id = parseInt(cb.parentElement.parentElement.querySelector('button')?.onclick.toString().match(/\d+/)?.[0]);
    if (id) {
      if (checkbox.checked) selectedRecommendations.add(id);
      else selectedRecommendations.delete(id);
    }
  });
  updateBulkActionsVisibility();
}

function toggleRecommendation(id, checked) {
  if (checked) selectedRecommendations.add(id);
  else selectedRecommendations.delete(id);
  updateBulkActionsVisibility();
}

function updateBulkActionsVisibility() {
  const bulkActions = document.getElementById('bulk-actions');
  if (selectedRecommendations.size > 0) {
    bulkActions.classList.remove('hidden');
    bulkActions.classList.add('flex');
  } else {
    bulkActions.classList.add('hidden');
    bulkActions.classList.remove('flex');
  }
}
