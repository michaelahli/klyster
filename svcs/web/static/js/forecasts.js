// Forecasts page
let selectedForecast = null;

function renderForecasts(container) {
  container.innerHTML = `
    <div class="space-y-6">
      <!-- Filters -->
      <div class="bg-white dark:bg-gray-800 rounded-lg p-4">
        <div class="flex flex-wrap gap-4">
          <div>
            <label class="block text-sm font-medium mb-1 dark:text-gray-300">Metric</label>
            <input type="text" id="filter-metric" placeholder="Filter by metric" class="px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white" />
          </div>
          <div>
            <label class="block text-sm font-medium mb-1 dark:text-gray-300">Date Range</label>
            <select id="filter-range" class="px-3 py-2 border rounded dark:bg-gray-700 dark:border-gray-600 dark:text-white">
              <option value="7">Last 7 days</option>
              <option value="30" selected>Last 30 days</option>
              <option value="90">Last 90 days</option>
            </select>
          </div>
          <div class="flex items-end">
            <button onclick="applyForecastFilters()" class="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700">Apply</button>
          </div>
        </div>
      </div>

      <!-- Forecasts List -->
      <div class="bg-white dark:bg-gray-800 rounded-lg p-6">
        <h3 class="text-lg font-semibold mb-4 dark:text-white">Forecasts</h3>
        <div id="forecasts-list" class="space-y-4">
          <div class="animate-pulse bg-gray-200 dark:bg-gray-700 h-20 rounded"></div>
        </div>
      </div>

      <!-- Forecast Detail Chart -->
      <div id="forecast-detail" class="hidden bg-white dark:bg-gray-800 rounded-lg p-6">
        <div class="flex justify-between items-center mb-4">
          <h3 class="text-lg font-semibold dark:text-white">Forecast Details</h3>
          <button onclick="closeForecastDetail()" class="text-gray-500 hover:text-gray-700 dark:text-gray-400 dark:hover:text-gray-200">✕</button>
        </div>
        <div id="forecast-metadata" class="mb-4 grid grid-cols-2 md:grid-cols-4 gap-4 text-sm"></div>
        <div class="h-96">
          <canvas id="forecast-chart"></canvas>
        </div>
        <div class="mt-4 flex gap-2">
          <button onclick="exportForecastCSV()" class="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700">Export CSV</button>
        </div>
      </div>
    </div>
  `;
  
  loadForecasts();
}

function applyForecastFilters() {
  loadForecasts();
}

async function loadForecasts() {
  const metric = document.getElementById('filter-metric')?.value || '';
  const range = document.getElementById('filter-range')?.value || '30';
  
  let url = '/api/v1/forecasts';
  const params = new URLSearchParams();
  if (metric) params.append('metric_name', metric);
  if (range) params.append('days', range);
  if (params.toString()) url += '?' + params.toString();
  
  try {
    const response = await fetch(url);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = await response.json();
    renderForecastsList(data.forecasts || []);
  } catch (error) {
    document.getElementById('forecasts-list').innerHTML = `
      <div class="bg-red-50 dark:bg-red-900/20 border border-red-200 dark:border-red-800 rounded-lg p-4">
        <p class="text-red-800 dark:text-red-200">Failed to load forecasts: ${error.message}</p>
        <button onclick="loadForecasts()" class="mt-2 px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700">Retry</button>
      </div>
    `;
  }
}

function renderForecastsList(forecasts) {
  const list = document.getElementById('forecasts-list');
  if (!forecasts.length) {
    list.innerHTML = '<p class="text-gray-500 dark:text-gray-400">No forecasts available</p>';
    return;
  }
  list.innerHTML = forecasts.map(f => `
    <div onclick="selectForecast(${f.id})" class="border dark:border-gray-700 rounded p-4 hover:bg-gray-50 dark:hover:bg-gray-700 cursor-pointer transition">
      <div class="flex justify-between items-start">
        <div>
          <div class="font-semibold dark:text-white">${f.metric_name}</div>
          <div class="text-sm text-gray-500 dark:text-gray-400 mt-1">
            ${f.model_name} · ${f.confidence_score?.toFixed(1) || 'N/A'}% confidence
          </div>
        </div>
        <div class="text-xs text-gray-400">
          ${new Date(f.created_at).toLocaleDateString()}
        </div>
      </div>
      <div class="mt-2 text-xs text-gray-500 dark:text-gray-400">
        Horizon: ${f.horizon_days || 'N/A'} days
      </div>
    </div>
  `).join('');
}

async function selectForecast(id) {
  try {
    const response = await fetch(`/api/v1/forecasts/${id}`);
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    const data = await response.json();
    selectedForecast = data;
    renderForecastDetail(data);
  } catch (error) {
    alert(`Failed to load forecast details: ${error.message}`);
  }
}

function renderForecastDetail(data) {
  const { forecast, points } = data;
  
  // Show detail section
  document.getElementById('forecast-detail').classList.remove('hidden');
  
  // Render metadata
  const metadata = document.getElementById('forecast-metadata');
  metadata.innerHTML = `
    <div class="dark:text-gray-300">
      <div class="text-gray-500 dark:text-gray-400 text-xs">Metric</div>
      <div class="font-semibold">${forecast.metric_name}</div>
    </div>
    <div class="dark:text-gray-300">
      <div class="text-gray-500 dark:text-gray-400 text-xs">Model</div>
      <div class="font-semibold">${forecast.model_name}</div>
    </div>
    <div class="dark:text-gray-300">
      <div class="text-gray-500 dark:text-gray-400 text-xs">Confidence</div>
      <div class="font-semibold">${forecast.confidence_score?.toFixed(1) || 'N/A'}%</div>
    </div>
    <div class="dark:text-gray-300">
      <div class="text-gray-500 dark:text-gray-400 text-xs">Horizon</div>
      <div class="font-semibold">${forecast.horizon_days || 'N/A'} days</div>
    </div>
  `;
  
  // Render chart
  renderForecastChart(forecast, points);
}

function closeForecastDetail() {
  document.getElementById('forecast-detail').classList.add('hidden');
  selectedForecast = null;
}

let forecastChart = null;

function renderForecastChart(forecast, points) {
  const ctx = document.getElementById('forecast-chart');
  
  // Destroy existing chart
  if (forecastChart) {
    forecastChart.destroy();
  }
  
  // Separate historical and forecast points
  const historical = points.filter(p => !p.is_forecast);
  const forecasted = points.filter(p => p.is_forecast);
  
  // Prepare data
  const historicalData = historical.map(p => ({
    x: new Date(p.timestamp),
    y: p.value
  }));
  
  const forecastData = forecasted.map(p => ({
    x: new Date(p.timestamp),
    y: p.value
  }));
  
  const lowerBound = forecasted.map(p => ({
    x: new Date(p.timestamp),
    y: p.lower_bound || p.value
  }));
  
  const upperBound = forecasted.map(p => ({
    x: new Date(p.timestamp),
    y: p.upper_bound || p.value
  }));
  
  forecastChart = new Chart(ctx, {
    type: 'line',
    data: {
      datasets: [
        {
          label: 'Historical',
          data: historicalData,
          borderColor: 'rgb(59, 130, 246)',
          backgroundColor: 'rgba(59, 130, 246, 0.1)',
          borderWidth: 2,
          pointRadius: 2,
          tension: 0.1
        },
        {
          label: 'Forecast',
          data: forecastData,
          borderColor: 'rgb(249, 115, 22)',
          backgroundColor: 'rgba(249, 115, 22, 0.1)',
          borderWidth: 2,
          borderDash: [5, 5],
          pointRadius: 2,
          tension: 0.1
        },
        {
          label: 'Lower Bound',
          data: lowerBound,
          borderColor: 'rgba(249, 115, 22, 0.3)',
          backgroundColor: 'rgba(249, 115, 22, 0.05)',
          borderWidth: 1,
          pointRadius: 0,
          fill: false,
          tension: 0.1
        },
        {
          label: 'Upper Bound',
          data: upperBound,
          borderColor: 'rgba(249, 115, 22, 0.3)',
          backgroundColor: 'rgba(249, 115, 22, 0.15)',
          borderWidth: 1,
          pointRadius: 0,
          fill: '-1',
          tension: 0.1
        }
      ]
    },
    options: {
      responsive: true,
      maintainAspectRatio: false,
      interaction: {
        mode: 'index',
        intersect: false,
      },
      plugins: {
        legend: {
          position: 'top',
        },
        tooltip: {
          callbacks: {
            title: (items) => {
              return items[0].parsed.x ? new Date(items[0].parsed.x).toLocaleString() : '';
            },
            label: (context) => {
              return `${context.dataset.label}: ${context.parsed.y.toFixed(2)}`;
            }
          }
        },
        zoom: {
          pan: {
            enabled: true,
            mode: 'x',
          },
          zoom: {
            wheel: {
              enabled: true,
            },
            pinch: {
              enabled: true
            },
            mode: 'x',
          }
        }
      },
      scales: {
        x: {
          type: 'time',
          time: {
            tooltipFormat: 'PP',
          },
          title: {
            display: true,
            text: 'Time'
          }
        },
        y: {
          title: {
            display: true,
            text: forecast.metric_name
          },
          beginAtZero: false
        }
      }
    }
  });
}

function exportForecastCSV() {
  if (!selectedForecast) return;
  
  const { forecast, points } = selectedForecast;
  
  // CSV header
  let csv = 'Timestamp,Value,Is Forecast,Lower Bound,Upper Bound\n';
  
  // CSV rows
  points.forEach(p => {
    const timestamp = new Date(p.timestamp).toISOString();
    const lower = p.lower_bound !== null ? p.lower_bound : '';
    const upper = p.upper_bound !== null ? p.upper_bound : '';
    csv += `${timestamp},${p.value},${p.is_forecast},${lower},${upper}\n`;
  });
  
  // Download
  const blob = new Blob([csv], { type: 'text/csv' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = `forecast_${forecast.id}_${forecast.metric_name}_${Date.now()}.csv`;
  a.click();
  URL.revokeObjectURL(url);
}
