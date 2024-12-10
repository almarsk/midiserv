import React, { useState } from "react";
import { Knob, KnobChangeEvent } from "primereact/knob";

type KnobControlProps = {
  socket: WebSocket;
  labelText: string;
  cc: number;
};

const KnobControl: React.FC<KnobControlProps> = ({ socket, labelText, cc }) => {
  const [value, setValue] = useState(0);

  const handleChange = (e: KnobChangeEvent) => {
    if (e.value == value) {
      return;
    }
    setValue(e.value);
    const buffer = new ArrayBuffer(2);
    const view = new DataView(buffer);
    view.setInt8(0, cc);
    view.setInt8(1, e.value);
    socket.send(buffer);
  };

  return (
    <>
      <div>{labelText}</div>
      <Knob value={value} onChange={handleChange} min={0} max={127} />
    </>
  );
};

export default KnobControl;
