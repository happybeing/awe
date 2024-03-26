<script>
export let webViewTitle="Autonomi Browser Webview";
export let webViewContent = "<HTML><HEAD></HEAD><BODY><h1>Default</h1> content</BODY></HTML>";
export let webViewUrl = 'http://dweb.happybeing.com';

</script>

<div style="">
  <iframe style="width: 100%" src={webViewUrl} name="webview" title={webViewTitle} />
</div>
