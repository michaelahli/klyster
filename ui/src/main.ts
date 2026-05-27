import App from './App.svelte';
import { mount } from 'svelte';
import './styles/app.css';

const target = document.getElementById('app');
if (!target) {
  throw new Error('expected #app element in index.html');
}

const app = mount(App, { target });

export default app;
