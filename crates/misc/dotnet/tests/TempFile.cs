using System;
using System.IO;

namespace Wasmtime.Tests
{
    internal class TempFile : IDisposable
    {
        public TempFile()
        {
            Path = System.IO.Path.GetTempFileName();
        }

        public void Dispose()
        {
            if (Path != null)
            {
                File.Delete(Path);
                Path = null;
            }
        }

        public string Path { get; private set; }
    }
}