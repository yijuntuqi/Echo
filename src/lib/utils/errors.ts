/**
 * Map backend error strings to friendly Chinese messages.
 *
 * The Rust side returns raw English error text ("daily quota exhausted",
 * reqwest timeouts, HTTP statuses). Shown verbatim those read like a crash
 * in a pet app; the mapping also turns the quota error into guidance toward
 * the user's own API key. Unknown errors pass through unchanged so real
 * bugs stay debuggable.
 */
export function friendlyError(e: unknown): string {
  const raw = typeof e === 'string' ? e : e instanceof Error ? e.message : String(e);
  const s = raw.toLowerCase();

  // Shared daily quota: the one error worth a call to action.
  if (s.includes('quota')) {
    return '今日共享额度用完啦（每日 200 次）～ 在「设置 → AI 设置」填入自己的 API Key 就能继续聊，且不再限量。';
  }
  if (s.includes('401') || s.includes('unauthorized') || (s.includes('invalid') && s.includes('key'))) {
    return 'API Key 好像不对劲，去「设置 → AI 设置」检查一下吧。';
  }
  if (s.includes('429') || s.includes('rate limit')) {
    return '说话太快啦，休息几秒再试试？';
  }
  if (s.includes('402') || s.includes('insufficient') || s.includes('balance')) {
    return '这个 Key 的余额不足了，去服务商那边看看吧。';
  }
  if (s.includes('timed out') || s.includes('timeout') || s.includes('deadline')) {
    return '回应超时了，网络可能不太顺畅，稍后再试试。';
  }
  if (s.includes('connect') || s.includes('network') || s.includes('dns')) {
    return '连不上服务器，请检查网络（或代理）后再试。';
  }
  if (s.includes('500') || s.includes('502') || s.includes('503') || s.includes('server error')) {
    return '服务器开小差了，等一会儿再试吧。';
  }
  return raw;
}
