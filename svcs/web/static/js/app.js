// Router
const routes = {
  '/': 'dashboard',
  '/resources': 'resources',
  '/forecasts': 'forecasts',
  '/recommendations': 'recommendations',
  '/analytics': 'analytics',
  '/settings': 'settings',
};

function navigate() {
  const hash = window.location.hash.slice(1) || '/';
  const route = routes[hash] || 'notfound';
  
  document.querySelectorAll('.nav-link').forEach(link => {
    link.classList.toggle('bg-gray-800', link.getAttribute('href') === `#${hash}`);
  });
  
  const content = document.getElementById('content');
  const title = document.getElementById('page-title');
  
  if (route === 'dashboard') {
    title.textContent = 'Dashboard';
    renderDashboard(content);
  } else if (route === 'forecasts') {
    title.textContent = 'Forecasts';
    renderForecasts(content);
  } else if (route === 'recommendations') {
    title.textContent = 'Recommendations';
    renderRecommendations(content);
  } else if (route === 'resources') {
    title.textContent = 'Resources';
    renderResources(content);
  } else if (route === 'settings') {
    title.textContent = 'Settings';
    renderSources(content);
  } else {
    title.textContent = route.charAt(0).toUpperCase() + route.slice(1);
    content.innerHTML = `<p class="text-gray-500">Coming soon: ${route}</p>`;
  }
}

window.addEventListener('hashchange', navigate);
window.addEventListener('load', navigate);
