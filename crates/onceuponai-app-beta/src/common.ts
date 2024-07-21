import { invoke } from "@tauri-apps/api/core";
import * as pluginHttp from '@tauri-apps/plugin-http';

export async function fetch(endpoint: String, options: any = {}) {

  const config: any = await invoke("config");
  const url = `${config.base_url}${endpoint}`;
  const defaultHeaders = {
  'Content-Type': 'application/json',
  'Authorization': `Bearer ${config.personal_token}`
  };

  const headers = { ...defaultHeaders, ...options.headers };

  return pluginHttp.fetch(url, {
    ...options,
    headers,
  })

}

export function isMobile() {
    return /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);
}

export function setCookie(name: string, value: string, days: number): void {
  let expires = "";
  if (days) {
    const date = new Date();
    date.setTime(date.getTime() + (days * 24 * 60 * 60 * 1000));
    expires = "; expires=" + date.toUTCString();
  }
  document.cookie = name + "=" + value + expires + "; path=/";
}

export function getCookie(name: string): string | null {
  const value = `; ${document.cookie}`;
  const parts = value.split(`; ${name}=`);
  if (parts.length === 2) {
    return parts.pop()?.split(';').shift() || null;
  }
  return null;
}

export function parseBool(value: string | null): boolean {
  return value?.toLowerCase() === "true";
}

export function deleteCookie(name: string): void {
  setCookie(name, "", -1);
}