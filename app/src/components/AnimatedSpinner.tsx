import React, { useEffect, useState } from "react";
import { Text } from "ink";
import Spinner from "ink-spinner";

interface AnimatedSpinnerProps {
  color?: string;
}

// A dedicated component for spinner animations
// This isolates re-renders to just this component when animation frames change
const AnimatedSpinner: React.FC<AnimatedSpinnerProps> = ({
  color = "gray",
}) => {
  // Force component updates at animation framerate without affecting parent components
  const [, setFrame] = useState(0);

  useEffect(() => {
    // Create a separate animation frame loop just for this component
    // This ensures only the spinner re-renders on animation frames
    let frameCount = 0;
    const intervalId = setInterval(() => {
      frameCount = (frameCount + 1) % 100; // Cycle frame count to avoid growing too large
      setFrame(frameCount);
    }, 100); // Reasonable frame rate for terminal spinner

    return () => clearInterval(intervalId);
  }, []);

  return (
    <Text color={color}>
      <Spinner type="dots" />
    </Text>
  );
};

export default React.memo(AnimatedSpinner);
