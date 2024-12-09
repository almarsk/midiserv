import React, { useState } from "react";
import { Knob, KnobChangeEvent } from "primereact/knob";

type KnobControlProps = {
  labelText: string;
  cc: number;
};

const KnobControl: React.FC<KnobControlProps> = ({ labelText, cc }) => {
  const [value, setValue] = useState(0);

  const handleChange = (e: KnobChangeEvent) => {
    setValue(e.value);
    console.log("emitting ", e.value, " to ", cc);
  };

  return (
    <>
      <div>{labelText}</div>
      <Knob value={value} onChange={handleChange} min={0} max={127} />
    </>
  );
};

export default KnobControl;
