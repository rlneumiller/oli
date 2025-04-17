import React from "react";
import { Box, Text } from "ink";
import theme from "../styles/gruvbox.js";

/**
 * A simple header component with left alignment and minimal styling
 */
const HeaderBox = React.memo(function HeaderBox({
  modelName,
}: {
  modelName: string;
}) {
  return (
    <Box
      width={40}
      alignSelf="flex-start"
      paddingX={1}
      paddingY={0}
      borderStyle="single"
      borderColor={theme.colors.dark.green}
      marginBottom={1}
    >
      <Text color={theme.colors.dark.green} bold>
        oli • {modelName}
      </Text>
    </Box>
  );
});

export default HeaderBox;
