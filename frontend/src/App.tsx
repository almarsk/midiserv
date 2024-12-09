import "./App.css";
import KnobControl from "./controls/KnobControl";

function App() {
  //const config = window.config || {};

  const socket = new WebSocket("ws://127.0.0.1:3000/ws");

  socket.onopen = () => {
    console.log("WebSocket connection established");
  };

  socket.onclose = () => {
    console.log("WebSocket connection closed");
  };

  return (
    <>
      <h1>jam with me - turn the knobs!</h1>
      <KnobControl socket={socket} labelText="yeah" cc={2} />
    </>
  );
}

export default App;
