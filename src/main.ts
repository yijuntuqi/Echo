// Main window entry (plain Svelte 5 + Vite, no SvelteKit)
import './lib/styles/global.css';
import { mount } from 'svelte';
import App from './App.svelte';

const target = document.getElementById('app');
if (!target) throw new Error('#app mount target not found');

export default mount(App, { target });
