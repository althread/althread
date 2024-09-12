// @refresh granular

import { createSignal, onCleanup, onMount } from "solid-js";
import init, { compile, run } from '../pkg/althread_web';
import Editor from './Editor';

init().then(() => {
  console.log('loaded');
});

export default function App() {
  

  return (
    <>
      <h1>Althread Editor</h1>
      <Editor
      compile={compile}
      run={run}
      onValueChange={(v) => {
        try{
          //console.log(compile(v)) 
        } catch(e) {
          //console.error(e);
        }
      }}/>
    </>
  );
}