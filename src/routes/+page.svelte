<script>
import logo from '../images/icon.png';
import AddressBar from '../lib/AddressBar.svelte'
import Navigation from '../lib/Navigation.svelte'
import WebsiteView from '../lib/WebsiteView.svelte'

import { webAddressStore } from '$lib/stores.js'

//$: webViewContent = "<HTML><HEAD></HEAD><BODY><h1>Main page</h1>content is address: " + $webAddressStore + "</BODY></HTML>";
let webViewContent = "EMPTY";

let webViewUrl = 'http://autonomi.com';
$: webViewUrl = $webAddressStore;

</script>

<div style="display:block; float:left;">
    <div>
        <Navigation/>
        <AddressBar/>
    </div>
</div>
<WebsiteView {webViewContent} {webViewUrl}/>


