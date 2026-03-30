import asyncio
from neurograph import NeuroGraph

async def main():
    print("🧠 Initializing NeuroGraph...")
    # Initialize NeuroGraph with default embedded storage
    ng = NeuroGraph()

    print("\n📝 Ingesting documents...")
    await ng.add("Alice is an engineer working at DeepMind on AGI.")
    await ng.add("DeepMind was acquired by Google.")
    await ng.add("Bob works with Alice at DeepMind.")

    print("\n🔍 Running hybrid queries...")
    
    # 1. Simple semantic query
    result1 = await ng.query("Who works at DeepMind?")
    print(f"Result 1 (Who works at DeepMind?): {result1}")

    # 2. Multi-hop reasoning query
    result2 = await ng.query("Who is Alice's employer owned by?")
    print(f"Result 2 (Employer owner): {result2}")

    # 3. Community Detection execution
    print("\n🌐 Running Community Detection (Louvain)...")
    await ng.detect_communities()
    
    print("\n🚀 Opening Interactive Dashboard...")
    # Automatically launch the temporal dashboard
    await ng.dashboard()

if __name__ == "__main__":
    asyncio.run(main())
