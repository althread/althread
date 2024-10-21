import { render } from 'solid-js/web';
import './main.css'
import App from './App';

const app = document.getElementById('app');

if (app) {
  render(() => <App />, app);
}