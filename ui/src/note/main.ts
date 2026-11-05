import { mount } from "svelte";
import Note from "./Note.svelte";
import "./note.css";

export default mount(Note, { target: document.getElementById("app")! });
