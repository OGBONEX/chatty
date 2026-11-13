import React from 'react';
import ReactDOM from 'react-dom/client';
import { Capacitor } from '@capacitor/core';
import App from './App';
import './styles.css';

if (!Capacitor.isNativePlatform() && import.meta.env.PROD && 'serviceWorker' in navigator) {
  window.addEventListener('load', () => {
    navigator.serviceWorker.register('/sw.js').catch((error) => {
      console.error('Chatty service worker registration failed:', error);
    });
  });
}

ReactDOM.createRoot(document.getElementById('root')!).render(
  <React.StrictMode><App /></React.StrictMode>,
);
