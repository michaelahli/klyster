// Mobile menu toggle
const mobileMenuBtn = document.getElementById('mobile-menu-btn');
const sidebar = document.getElementById('sidebar');
const sidebarBackdrop = document.getElementById('sidebar-backdrop');

if (mobileMenuBtn) {
  mobileMenuBtn.addEventListener('click', () => {
    sidebar.classList.toggle('-translate-x-full');
    sidebarBackdrop.classList.toggle('hidden');
  });
}

if (sidebarBackdrop) {
  sidebarBackdrop.addEventListener('click', () => {
    sidebar.classList.add('-translate-x-full');
    sidebarBackdrop.classList.add('hidden');
  });
}

// Theme toggle
const themeToggle = document.getElementById('theme-toggle');
let theme = localStorage.getItem('theme') || 'light';

function applyTheme(t) {
  document.documentElement.classList.toggle('dark', t === 'dark');
  themeToggle.textContent = t === 'dark' ? '☀️' : '🌙';
}

applyTheme(theme);

themeToggle.addEventListener('click', () => {
  theme = theme === 'dark' ? 'light' : 'dark';
  localStorage.setItem('theme', theme);
  applyTheme(theme);
});

// Toast notifications
function showToast(message, type = 'info') {
  const toast = document.createElement('div');
  const bgColor = type === 'success' ? 'bg-green-600' : type === 'error' ? 'bg-red-600' : 'bg-blue-600';
  toast.className = `fixed bottom-4 right-4 ${bgColor} text-white px-6 py-3 rounded-lg shadow-lg z-50 transition-opacity duration-300`;
  toast.textContent = message;
  document.body.appendChild(toast);
  
  setTimeout(() => {
    toast.style.opacity = '0';
    setTimeout(() => toast.remove(), 300);
  }, 3000);
}

// Confirmation modal
function showConfirmModal(title, message, onConfirm) {
  const backdrop = document.createElement('div');
  backdrop.className = 'fixed inset-0 bg-black bg-opacity-50 z-50 flex items-center justify-center';
  backdrop.innerHTML = `
    <div class="bg-white dark:bg-gray-800 rounded-lg p-6 max-w-md mx-4 shadow-xl">
      <h3 class="text-lg font-semibold mb-2 dark:text-white">${title}</h3>
      <p class="text-gray-600 dark:text-gray-300 mb-4">${message}</p>
      <div class="flex gap-2 justify-end">
        <button id="modal-cancel" class="px-4 py-2 bg-gray-200 dark:bg-gray-700 text-gray-800 dark:text-gray-200 rounded hover:bg-gray-300 dark:hover:bg-gray-600">Cancel</button>
        <button id="modal-confirm" class="px-4 py-2 bg-red-600 text-white rounded hover:bg-red-700">Confirm</button>
      </div>
    </div>
  `;
  document.body.appendChild(backdrop);
  
  backdrop.querySelector('#modal-cancel').addEventListener('click', () => backdrop.remove());
  backdrop.querySelector('#modal-confirm').addEventListener('click', () => {
    backdrop.remove();
    onConfirm();
  });
  backdrop.addEventListener('click', (e) => {
    if (e.target === backdrop) backdrop.remove();
  });
}
