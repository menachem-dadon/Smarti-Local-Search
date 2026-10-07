const fs=require('node:fs');const path=require('node:path');
const React=require('../desktop/node_modules/react');const {renderToStaticMarkup}=require('../desktop/node_modules/react-dom/server');const {IconFileSearch}=require('../desktop/node_modules/@tabler/icons-react');
const svg=renderToStaticMarkup(React.createElement(IconFileSearch,{size:256,stroke:2,color:'#365ccd'}));
const out=path.join(__dirname,'../desktop/src-tauri/icons');fs.mkdirSync(out,{recursive:true});fs.writeFileSync(path.join(out,'icon.svg'),svg);
const {Resvg}=require('../desktop/node_modules/@resvg/resvg-js');fs.writeFileSync(path.join(out,'icon.png'),new Resvg(svg,{background:'rgba(0,0,0,0)'}).render().asPng());
