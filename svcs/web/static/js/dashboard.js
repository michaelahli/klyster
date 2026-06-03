// Dashboard rendering
function renderDashboard(container) {
  container.innerHTML = `
    <div class="space-y-6">
      <div id="metrics-grid" class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
        <div class="animate-pulse bg-white rounded-lg p-4 h-32"></div>
        <div class="animate-pulse bg-white rounded-lg p-4 h-32"></div>
        <div class="animate-pulse bg-white rounded-lg p-4 h-32"></div>
        <div class="animate-pulse bg-white rounded-lg p-4 h-32"></div>
      </div>
      <div class="bg-white dark:bg-gray-800 rounded-lg p-6">
        <h3 class="text-lg font-semibold mb-4 dark:text-white">CPU Usage (24h)</h3>
        <canvas id="cpu-chart" height="80"></canvas>
      </div>
    </div>
  `;
  
  loadMetrics();
}

async function loadMetrics() {
  try {
    const response = await fetch('/api/v1/metrics/latest');
    const data = await response.json();
    const metrics = Array.isArray(data) ? data : (data.names || data.data || []);
    renderMetricCards(metrics);
    loadCpuChart();
  } catch (error) {
    document.getElementById('metrics-grid').innerHTML = `
      <div class="col-span-4 bg-red-50 border border-red-200 rounded-lg p-4">
        <p class="text-red-800">Failed to load metrics: ${error.message}</p>
        <button onclick="loadMetrics()" class="mt-2 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700">Retry</button>
      </div>
    `;
  }
}

function renderMetricCards(metrics) {
  const grid = document.getElementById('metrics-grid');
  const cards = metrics.map(m => createMetricCard(m)).join('');
  grid.innerHTML = cards;
}

function createMetricCard(metric) {
  const percentage = metric.value;
  let colorClass = 'text-green-600';
  if (percentage >= 85) colorClass = 'text-red-600';
  else if (percentage >= 70) colorClass = 'text-yellow-600';
  
  return `
    <div class="bg-white dark:bg-gray-800 rounded-lg p-4 shadow">
      <div class="text-sm text-gray-500 dark:text-gray-400 mb-1">${metric.name}</div>
      <div class="text-3xl font-bold ${colorClass}">${percentage.toFixed(1)}%</div>
      <div class="text-xs text-gray-400 mt-1">${new Date(metric.timestamp).toLocaleString()}</div>
    </div>
  `;
}

async function loadCpuChart() {
  try {
    const response = await fetch('/api/v1/metrics/cpu_usage?hours=24');
    const data = await response.json();
    
    const ctx = document.getElementById('cpu-chart');
    new Chart(ctx, {
      type: 'line',
      data: {
        labels: data.map(d => new Date(d.timestamp).toLocaleTimeString()),
        datasets: [{
          label: 'CPU Usage (%)',
          data: data.map(d => d.value),
          borderColor: 'rgb(59, 130, 246)',
          tension: 0.1
        }]
      },
      options: {
        responsive: true,
        maintainAspectRatio: false
      }
    });
  } catch (error) {
    console.error('Failed to load CPU chart:', error);
  }
}
