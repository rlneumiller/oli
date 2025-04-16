// Gruvbox color palette
export const colors = {
  // Dark theme colors
  dark: {
    // Background colors
    bg: '#282828',
    bg0: '#282828',
    bg1: '#3c3836',
    bg2: '#504945',
    bg3: '#665c54',
    bg4: '#7c6f64',
    
    // Foreground colors
    fg: '#ebdbb2',
    fg0: '#fbf1c7',
    fg1: '#ebdbb2',
    fg2: '#d5c4a1',
    fg3: '#bdae93',
    fg4: '#a89984',
    
    // Accent colors
    red: '#fb4934',
    green: '#b8bb26',
    yellow: '#fabd2f',
    blue: '#83a598',
    purple: '#d3869b',
    aqua: '#8ec07c',
    orange: '#fe8019',
    
    // Muted accent colors
    darkRed: '#cc241d',
    darkGreen: '#98971a',
    darkYellow: '#d79921',
    darkBlue: '#458588',
    darkPurple: '#b16286',
    darkAqua: '#689d6a',
    darkOrange: '#d65d0e',
    
    // Grayscale
    gray: '#928374',
  },
  
  // Light theme colors (not used for now)
  light: {
    // Background colors
    bg: '#fbf1c7',
    bg0: '#fbf1c7',
    bg1: '#ebdbb2',
    bg2: '#d5c4a1',
    bg3: '#bdae93',
    bg4: '#a89984',
    
    // Foreground colors
    fg: '#3c3836',
    fg0: '#282828',
    fg1: '#3c3836',
    fg2: '#504945',
    fg3: '#665c54',
    fg4: '#7c6f64',
    
    // Accent colors
    red: '#9d0006',
    green: '#79740e',
    yellow: '#b57614',
    blue: '#076678',
    purple: '#8f3f71',
    aqua: '#427b58',
    orange: '#af3a03',
    
    // Bright accent colors
    brightRed: '#cc241d',
    brightGreen: '#98971a',
    brightYellow: '#d79921',
    brightBlue: '#458588',
    brightPurple: '#b16286',
    brightAqua: '#689d6a',
    brightOrange: '#d65d0e',
    
    // Grayscale
    gray: '#928374',
  }
};

// Common UI components styling
export const styles = {
  // Box styles
  box: {
    default: {
      borderColor: colors.dark.gray,
    },
    header: {
      borderStyle: "single" as const, // Type assertion to make it compatible with Ink
      borderColor: colors.dark.green,
      paddingX: 2,
      paddingY: 1,
    },
    content: {
      borderStyle: "single" as const,
      borderColor: colors.dark.gray,
    },
    input: {
      borderStyle: "single" as const, 
      borderColor: colors.dark.gray,
    },
  },
  
  // Text styles
  text: {
    heading: {
      color: colors.dark.green,
      bold: true,
    },
    subheading: {
      color: colors.dark.yellow,
      bold: true,
    },
    user: {
      color: colors.dark.green,
      bold: true,
    },
    assistant: {
      color: colors.dark.blue,
    },
    system: {
      color: colors.dark.yellow,
      italic: true,
    },
    tool: {
      color: colors.dark.purple,
      bold: true,
    },
    highlight: {
      color: colors.dark.orange,
      bold: true,
    },
    dimmed: {
      color: colors.dark.fg4,
    },
    error: {
      color: colors.dark.red,
    },
    success: {
      color: colors.dark.green,
    },
    warning: {
      color: colors.dark.yellow,
    },
    info: {
      color: colors.dark.blue,
    },
  },
  
  // Status indicators
  status: {
    active: {
      color: colors.dark.green,
      icon: '●',
    },
    processing: {
      color: colors.dark.yellow,
      // Spinner will be used
    },
    error: {
      color: colors.dark.red,
      icon: '○',
    },
    inactive: {
      color: colors.dark.gray,
      icon: '○',
    },
  },
  
  // Button-like elements
  button: {
    primary: {
      color: colors.dark.bg0,
      backgroundColor: colors.dark.green,
      bold: true,
    },
    secondary: {
      color: colors.dark.bg0,
      backgroundColor: colors.dark.blue,
      bold: true,
    },
    danger: {
      color: colors.dark.bg0,
      backgroundColor: colors.dark.red,
      bold: true,
    },
  },
};

export default { colors, styles };