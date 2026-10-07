import '@testing-library/jest-dom/vitest';

// jsdom does not implement the browser's native dialog methods.
Object.defineProperties(HTMLDialogElement.prototype, {
  showModal: {configurable:true,writable:true,value:function(this:HTMLDialogElement){this.open=true}},
  close: {configurable:true,writable:true,value:function(this:HTMLDialogElement){this.open=false}},
});
