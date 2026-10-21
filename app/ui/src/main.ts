import { mount } from "svelte";
import icon from "../../icon.svg?inline";
import App from "./App.svelte";
import "./app.css";

// The app's icon for the browser tab of the single-file page, inlined so the page stays one file.
const link = Object.assign(document.createElement("link"), { rel: "icon", type: "image/svg+xml", href: icon });
document.head.append(link);

export default mount(App, { target: document.getElementById("app")! });
