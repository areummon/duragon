export const toast = $state<{ text: string; action: { label: string; run: () => void } | null; seq: number }>({
  text: '', action: null, seq: 0,
});

let timer = 0;

export function showToast(text: string, action: { label: string; run: () => void } | null = null) {
  toast.text = text;
  toast.action = action;
  toast.seq++;
  window.clearTimeout(timer);
  timer = window.setTimeout(hideToast, 6000);
}

export function hideToast() {
  window.clearTimeout(timer);
  toast.text = '';
  toast.action = null;
}
