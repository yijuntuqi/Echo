// Pet overlay window entry (transparent always-on-top window)
import './lib/styles/global.css';
import { mount } from 'svelte';
import PetWindow from './PetWindow.svelte';

const target = document.getElementById('app');
if (!target) throw new Error('#app mount target not found');

export default mount(PetWindow, { target });
