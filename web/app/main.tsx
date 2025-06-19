/** @jsxImportSource solid-js */
import { render } from 'solid-js/web';
import './assets/styles/main.css';
import { HashRouter, Route } from "@solidjs/router";
import App from './App';
import Tutorial from "./Tutorial";

const app = document.getElementById('app');

if (app) {
  render(() => 
    <HashRouter>
      <Route path="/tutorials" component={Tutorial} />
      <Route path="/" component={App} />
    </HashRouter>, app);
}