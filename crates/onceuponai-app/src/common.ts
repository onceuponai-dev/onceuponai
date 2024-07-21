import { invoke } from "@tauri-apps/api/tauri";
import axios from 'axios';
import axiosTauriApiAdapter from 'axios-tauri-api-adapter';

export async function axios_client() {
  const config: any = await invoke("config");
  const instance = axios.create({ adapter: axiosTauriApiAdapter, baseURL: config.base_url });
  instance.defaults.headers.common['Authorization'] = `Bearer ${config.personal_token}`;
  instance.defaults.timeout = 20000;
  return instance;
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