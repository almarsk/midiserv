import "./App.css";
import KnobControl from "./controls/KnobControl";

function App() {
  //const config = window.config || {};

  return (
    <>
      <h1>jam with me - turn the knobs!</h1>
      <KnobControl labelText="yeah" cc={2} />
    </>
  );
}

export default App;
