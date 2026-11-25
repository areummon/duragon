import '@fontsource-variable/instrument-sans';
import '@fontsource-variable/newsreader';
import './styles/tokens.css';
import './styles/motion.css';
import './styles/base.css';
import './styles/ui.css';
import './styles/editor.css';
import './styles/riddle.css';
import { mount } from 'svelte';
import App from './App.svelte';

export default mount(App, { target: document.getElementById('app')! });
