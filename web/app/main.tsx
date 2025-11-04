/** @jsxImportSource solid-js */
import { render } from 'solid-js/web';
import './main.css';
import { HashRouter, Route, useNavigate } from "@solidjs/router";
import { onMount } from 'solid-js';
import App from './App';
import Tutorial from "./components/tutorial/Tutorial";

// This component will handle the redirection for any unmatched route.
const NotFound = () => {
  const navigate = useNavigate();
  onMount(() => {
    navigate('/', { replace: true });
  });
  return null; // It renders nothing, just performs the redirect.
};

const app = document.getElementById('app');

if (app) {
  render(() => 
    <HashRouter>
      <Route path="/tutorials" component={Tutorial} />
      <Route path="/tutorials/:stepName" component={Tutorial} />
      {/* Main app route */}
      <Route path="/" component={App} />
      {/* Wildcard route must be last. It catches any path that wasn't matched above. */}
      <Route path="*all" component={NotFound} />
    </HashRouter>, app);
}